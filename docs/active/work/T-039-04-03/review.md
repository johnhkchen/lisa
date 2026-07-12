# Review: cross-policy deadline regression

## Outcome

Ticket `T-039-04-03` is implemented and verified.

Two deterministic cross-policy regressions now lock:

- the native action produced by each deadline family;
- the complete transition action set;
- action identity and payload;
- recent-activity behavior across policies;
- awaiting-human behavior across policies;
- session warning versus destructive reclaim;
- health's observational non-exemption;
- acknowledgement and transition-exit non-exemptions.

No production behavior changed.

## Source inventory

Modified:

- `crates/lisa-plugin/src/deadline.rs`.

Created source files:

- none.

Deleted source files:

- none.

No dependency, configuration, public interface, visibility, serialized schema,
or state-machine integration changed.

## Source commit

Committed through Lisa's isolated transaction:

```text
62234bc69c904ef7440d41e4af60b92a5c78948d
test(plugin): lock cross-policy deadline contracts
```

The commit contains exactly:

- `crates/lisa-plugin/src/deadline.rs`.

The ticket-owned source path is clean. The ordinary Git index is empty.
Remaining worktree entries are Lisa-managed ticket, provenance, and admitted
workflow artifact state.

## New exact-action test

`cross_policy_deadline_actions_remain_distinct` uses the injected fixed clock and
exercises all six deadline families in one comparative regression.

### Acknowledgement

The test asserts:

- the inclusive expired candidate is emitted;
- pane identity remains `11`;
- captured assignment state remains `"expired"`.

This protects the evaluator's captured-state action rather than only asserting
that some acknowledgement deadline fired.

### Transition

The test submits expired, quiet, non-human candidates for all three states and
asserts the exact ordered actions:

- `ExitReady` with pane and optional ticket identity;
- `StopTimedOut` with pane identity;
- `ClearTimedOut` with pane and optional ticket identity.

This fails if transition variants are collapsed, reordered, suppressed, or
their identity payload changes.

### Review

The test asserts the dedicated `ReviewAction` retains:

- ticket `T-REVIEW`;
- pane `31`.

It therefore distinguishes Review prompting from every reclaim-style result.

### Health

The test asserts a full observation transition:

- ticket `T-HEALTH`;
- previous `Healthy`;
- current `Stuck`.

This protects health as observational state rather than a destructive action.

### Session

The test asserts exact:

- `SessionAction::Reclaim` variant;
- ticket and pane identity;
- elapsed value of 100 seconds;
- Implement phase;
- global-budget precedence over the later phase clock.

The complete `SessionDeadline` payload is compared by equality.

### Stale

The test asserts the exact dedicated stale action with:

- ticket `T-STALE`;
- pane `51`.

## New exemption matrix test

`cross_policy_activity_and_human_exemptions_remain_distinct` compares the same
conditions across policy boundaries at one fixed instant.

| Policy | Recent activity | Awaiting human |
|---|---|---|
| acknowledgement | no evaluator exemption | no evaluator exemption |
| transition exit | `ExitReady` | `ExitReady` |
| transition stop | suppressed | suppressed |
| transition clear | suppressed | suppressed |
| review | suppressed | suppressed |
| health | `Healthy` | quiet pane remains `Stuck` |
| session | `Warn` | `Warn` |
| stale | suppressed | suppressed |

The test asserts exact identities so a candidate cannot enter or leave output
without producing a useful diff.

## Important policy distinctions preserved

### Suppression versus conversion

Review, transition stop/clear, and stale policies suppress exempt candidates.

Session does not suppress an expired budget. Recent activity or awaiting-human
converts destructive `Reclaim` into advisory `Warn`. The regression asserts both
warnings and the non-exempt reclaim in one ordered result.

### Observation versus exemption

Health does not accept awaiting-human as an evaluator input. A recently active
thread is Healthy, but a quiet thread remains Stuck even when the state layer
marks its pane awaiting human. The new test documents that deliberate absence,
and the unchanged state characterization proves the real marker remains present.

### Exit versus stop/clear

Transition exit ignores recent pane activity and awaiting-human after its grace
deadline. Stop and clear honor both exemptions. The new matrix includes all six
combinations and expects only the two exit actions.

### Acknowledgement recovery

Acknowledgement has no activity or awaiting-human evaluator fields. Its fixed
expiry action remains present. The unchanged state characterization additionally
proves that an awaiting-human assignment timeout still enters fresh recovery.

## Existing coverage retained

The six `characterizes_*` tests in `lib.rs` were not modified.

They continue to prove state-layer effects:

- acknowledgement recovery and lease replacement;
- exit cleanup and transition exemptions;
- Review finish-up activity;
- health cache/log observation;
- session fencing and timeout outcome;
- stale fencing and reclaim outcome.

Together, old and new coverage bracket the evaluator boundary:

```text
cross-policy evaluator contract
  -> typed action
  -> unchanged State application
  -> characterized state-machine effect
```

## Verification results

Focused new regression:

```text
cargo test -p lisa-plugin cross_policy_ --no-fail-fast
2 passed, 0 failed
```

All evaluator tests:

```text
cargo test -p lisa-plugin deadline::tests --no-fail-fast
4 passed, 0 failed
```

Unchanged deadline characterization:

```text
cargo test -p lisa-plugin characterizes_ --no-fail-fast
6 passed, 0 failed
```

Complete plugin suite:

```text
cargo test -p lisa-plugin --no-fail-fast
325 passed, 0 failed
```

Complete workspace suite:

```text
cargo test --workspace --no-fail-fast
all executed tests passed
CLI: 274 passed
core: 155 passed
plugin: 325 passed
```

Explicit lint gate:

```text
cargo clippy --workspace --all-targets --all-features -- -D warnings
passed
```

Repository gate:

```text
just check
WASM check passed
workspace tests passed
```

Formatting and whitespace:

```text
cargo fmt --all -- --check
passed

git diff --check
passed
```

Post-commit focused execution also passed both new tests.

## Acceptance assessment

The acceptance criterion is satisfied:

- cross-policy regression tests exist;
- they use a deterministic injected clock;
- every deadline family's action is directly asserted;
- all transition action variants are directly asserted;
- active/recent-session behavior is compared across applicable policies;
- awaiting-human behavior is compared across applicable policies;
- advisory, destructive, observational, and suppressed outcomes remain distinct;
- existing state-effect characterization remains unchanged and green;
- plugin and workspace suites are green;
- Clippy is green with warnings denied;
- WASM repository check is green.

## Coverage limitations

- Acknowledgement action application still depends on the assignment-state
  variant in `lib.rs`; that variant-specific recovery behavior is covered by
  existing state tests rather than duplicated in the evaluator matrix.
- Health cannot receive an awaiting-human flag by design. Its non-exemption is
  therefore jointly proved by the new quiet observation and the existing state
  characterization with the marker installed.
- The matrix focuses on the story-named activity and awaiting-human distinctions.
  Other eligibility fields such as thread status, pending completion, phase,
  zero-duration disabling, and already-prompted remain covered by existing unit
  and state tests.
- No live Zellij timing test was added. The story explicitly keeps live-seat
  timing observations outside this slice, and fixed-clock behavior is the right
  deterministic boundary here.

## Maintainability notes

- Fixtures are intentionally inline. This makes the policy-controlling input
  fields visible during review and avoids a generic builder that could hide
  differences.
- The tests are verbose because heterogeneous policy inputs and actions remain
  typed. That verbosity is useful regression evidence for this ticket.
- Adding a new exemption field or action payload requires an intentional update
  to the corresponding matrix block and expected result.
- The `cross_policy_` prefix provides a fast focused gate for future evaluator
  refactors.

## Open concerns

There is no critical issue, failing gate, uncommitted ticket-owned source, or
known acceptance gap.

The ignored real-Zellij test remains environment-gated and is unrelated to this
pure fixed-clock regression. Live-seat evidence remains deferred according to
the story boundary.

This review is ready for Lisa's final completion publication and commit gate.
