# Review: recorded Review livelock regression

## Disposition

Pass. The deterministic regression transcribes the T-009-01-01 field sequence,
demonstrates that a naive edge-triggered implementation reproduces the missed
request and stale finish-up prompt, and proves the settled completion aggregate
converges to exactly one Request and exactly one Confirmed transition without a
finish-up or re-request. The only ticket-owned source file is committed and all
verification is green.

## Commit

```text
e28d71209ae5cb2722894c96e29f596e3d7df7a9
test(core): replay recorded completion livelock
```

The commit contains exactly:

```text
crates/lisa-core/tests/recorded_livelock_regression.rs
```

The commit adds 220 lines in one new integration test. No production file,
manifest, lockfile, plugin, CLI, ticket, or shared work artifact is included.

## Acceptance result

The acceptance criterion is satisfied.

The deterministic test replays these exact milestones in order:

1. Review artifact exists before phase Review;
2. phase advances to Review;
3. stop is observed;
4. the recorded approximately ten-minute timeout elapses;
5. reload is observed;
6. the matching manual command result confirms completion.

The aggregate-backed replay asserts:

- exactly one completion Request;
- exactly one authoritative Confirmed transition;
- zero finish-up prompts while the artifact exists;
- zero completion re-requests;
- terminal `CompletionState::Confirmed`.

The same fixed event array is fed to the naive edge-triggered negative control.
That model exhibits the expected historical failure:

- zero aggregate requests because artifact creation preceded Review;
- one stale finish-up prompt at timeout;
- one later external manual confirmation.

The test explicitly asserts that the naive observation differs from the
required contract and pins its exact failure shape.

## File created

### `crates/lisa-core/tests/recorded_livelock_regression.rs`

This is a standalone integration test of lisa-core's public completion API. It
imports `reduce`, `reconcile`, the typed state/event/identity/effect values, and
the structured Review disposition. It does not access private module helpers.

The file contains:

- stable field-derived attempt, completion, and correlation identities;
- a typed `RecordedEvent` enum;
- a single visible fixed trace function;
- an `Observation` counter value;
- an aggregate-backed replay driver;
- a deliberately naive edge-triggered comparison stub;
- one deterministic regression test consuming both drivers.

## Aggregate replay assessment

The harness retains phase and artifact presence as fixture-side observations.
That is necessary because phase is intentionally not part of the pure
`DurableCompletionInputs` contract. Once phase Review is observed, the driver
constructs the admitted current-attempt artifact plus exact Pass disposition.

At the phase edge, `reconcile` sees the already-present durable artifact and
returns `LaunchCompletion`. The driver applies the corresponding Request through
`reduce`, requires Requested plus the identical effect, and counts one request.

The driver then applies CommandLaunched through `reduce` with the stable
correlation. Stop, timeout, and reload each reconcile the in-flight state. Each
returns the correlation-tagged actionable result and emits no request.

Finally, the manual result is represented as matching CommandSucceeded and is
applied through `reduce`. The driver requires Confirmed with no effect, counts
one authoritative confirmation, and reconciles the terminal state once more to
prove it emits no request.

All lifecycle transitions therefore pass through production aggregate code;
the harness does not hand-code state transitions.

## Finish-up assessment

`EffectCommand` deliberately has no pane-prompt variant and lisa-core has no
plugin adapter dependency. The test therefore counts finish-up as a synthetic
fixture-side adapter observation on the timeout milestone.

The policy is explicit: a timeout can count a finish-up only when the Review
artifact is absent. In the recorded trace the artifact has existed since the
first event, so the count remains zero.

This assertion proves the intended trace contract without claiming that the
current production plugin is already wired to the new reducer. Real runtime
finish-up suppression is owned by E-042.

## Naive-stub assessment

The negative control requests completion only on an artifact-created edge when
phase Review is already true. It intentionally does not re-inspect the artifact
on phase advance, stop, timeout, or reload.

Because the trace writes Review first, the edge is discarded. Phase advance
does not recover it, and timeout emits the unwanted finish-up. This is the
minimal executable form of the field livelock's edge-triggered failure mode.

The stub remains separate from the aggregate driver and calls neither
`reconcile` nor `reduce`. It is evidence that the fixture is discriminating,
not an alternative implementation offered for production.

## Test coverage

The new test directly covers:

- artifact-before-phase ordering;
- level-triggered eligibility on the later phase observation;
- exact request effect identity payload;
- transition to correlation-bearing CommandInFlight;
- duplicate suppression on stop;
- existing-artifact timeout prompt suppression;
- duplicate suppression on timeout;
- correlation retention across reload;
- matching manual-result confirmation;
- terminal Confirmed reconciliation suppression;
- naive missed-edge regression behavior.

Existing completion unit tests continue to cover missing admission, blocked and
invalid dispositions, retryable/action-required rejection, launch failure,
correlation mismatch, duplicate requests, and illegal callback edges.

## Verification results

The following passed after final formatting:

```text
cargo fmt --all -- --check
cargo test -p lisa-core --test recorded_livelock_regression
cargo test -p lisa-core
cargo clippy -p lisa-core --all-targets -- -D warnings
git diff --check -- crates/lisa-core/tests/recorded_livelock_regression.rs
cargo test --workspace
```

Observed results:

- deterministic integration regression: 1 passed, 0 failed;
- lisa-core unit tests: 191 passed, 0 failed;
- lisa-cli unit tests: 279 passed, 0 failed;
- CLI integration suites: 1 + 3 + 1 passed, 0 failed;
- lisa-plugin unit tests: 341 passed, 0 failed;
- core doctests: 0 failed;
- clippy: passed with warnings denied;
- formatting and diff hygiene: passed;
- real-Zellij integration: one existing environment-gated test ignored.

## Repository preservation

The meaningful source unit was committed only through `lisa commit-ticket`
with its exact repository-relative include path. Ordinary `git add`, broad add,
and ordinary `git commit` were not used.

Post-commit inspection confirms:

- the commit contains exactly the new integration test;
- the ticket-owned source path is clean;
- the ordinary index is empty;
- Lisa-managed ticket, provenance, and published artifact changes remain
  outside the source commit;
- unrelated untracked plugin documentation was not touched.

## Open concerns and limitations

No blocking concern was found.

This is intentionally a pure-domain fixture transcription. It does not execute
the real plugin scheduler, pane finish-up delivery, journal persistence,
operator `[d]one` path, isolated completion CLI transaction, seat release, or
dependent scheduling. Those runtime boundaries are explicitly assigned to
E-042 and its Arcade-shaped adapter/live regressions.

The replay retains aggregate state across the synthetic reload milestone. E-041
defines the aggregate contract; durable persistence or reconstruction of that
state is a follow-on journal concern. The test still proves repeated
reconciliation cannot duplicate an in-flight or confirmed request.

The manual result is modeled as the matching successful command result. It does
not claim that the historical manual CLI invocation was launched through the
aggregate; it proves the domain accepts one attributed confirming result and
ends in one authoritative Confirmed state.

The final release WASM build and size-budget gate is intentionally left to
T-041-02-03 after both deterministic and generated tests have landed. This
ticket adds no production dependency and its integration test is not part of
the WASM artifact.

## Critical issues requiring human attention

None.

## Human review focus

A reviewer should confirm that the fixture-side representation of phase and
finish-up policy preserves the honest pure-domain boundary while still making
the acceptance trace explicit. The core correctness claim itself is directly
backed by public reducer/reconciler transitions and exact counter assertions.

Review is complete. This attempt remains on T-041-02-01 and waits for Lisa to
admit the Review, prepare the final completion commit, publish Done, and release
the seat.

