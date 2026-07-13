# Plan: recorded Review livelock regression

## Step 1: create the standalone integration test

Create `crates/lisa-core/tests/recorded_livelock_regression.rs`. Import the
public completion types/functions and `ReviewDisposition` only.

Verification: Cargo discovers the new integration test without manifest edits.

Atomic outcome: the proof layer has a source location disjoint from settled
production code and the follow-on generated test module.

## Step 2: encode the recorded trace

Add a private typed event enum and an exact ordered trace covering artifact
before Review, phase advance, stop, ten-minute timeout, reload, and confirming
manual result.

Use stable T-009-01-01-derived attempt, completion, and correlation identities.

Verification: the event order is reviewable as one fixed array and includes all
milestones named by the acceptance criterion.

## Step 3: define observable evidence

Add a small observation value that independently records completion requests,
authoritative confirmations, finish-up prompts, re-requests, and terminal
confirmation.

Verification: equality assertions expose which invariant diverged rather than
reducing the result to a single boolean.

## Step 4: implement the aggregate fixture driver

Track Review phase and artifact presence as adapter-side fixture facts. Track
the real `CompletionState` as aggregate state. Construct durable admitted Pass
inputs and invoke `reconcile` after each safe Review observation.

Verification: the driver does not duplicate production state-transition logic;
every request, launch, and success edge passes through public `reduce`.

## Step 5: apply the single request

When reconciliation emits `LaunchCompletion`, apply the corresponding Request
event to Eligible. Assert the reducer returns Requested and the same exact
effect. Count one request.

Then apply CommandLaunched with the fixed correlation and assert
CommandInFlight with no effect.

Verification: effect payloads and correlation identity match exactly.

## Step 6: model timeout suppression and reload

On timeout, emit a synthetic finish-up only when the Review artifact is absent.
Because this trace's artifact already exists, no finish-up is recorded.

Reconcile on stop, timeout, and reload. Require in-flight reconciliation to
retain the exact correlation and emit no launch effect.

Verification: request remains exactly one and re-request remains zero across
all hostile observations.

## Step 7: apply the confirming result

Feed matching CommandSucceeded through the reducer. Require the exact
CommandInFlight-to-Confirmed transition with no effect and count one
authoritative confirmation.

Reconcile Confirmed once more and require no request.

Verification: final state is Confirmed and authoritative Done count is one.

## Step 8: implement the naive negative control

Add a separate minimal edge-triggered stub. Request only when artifact creation
arrives after phase Review. Do not revisit artifact presence on phase, stop, or
reload. Emit finish-up at timeout when the request was missed.

Verification: the same trace produces zero aggregate requests and one unwanted
finish-up, demonstrating why artifact-before-phase ordering defeats the stub.

## Step 9: assert the deterministic regression

In one test, feed the identical recorded event slice to both drivers. Assert
the naive observation differs from the desired contract and assert its exact
failure shape. Assert the aggregate observation exactly equals:

- one request;
- one authoritative confirmation;
- zero finish-up prompts;
- zero re-requests;
- Confirmed terminal state.

Verification: changing the fixture or contract yields a focused test failure.

## Step 10: run focused formatting and tests

Run:

```text
cargo fmt --all
cargo fmt --all -- --check
cargo test -p lisa-core --test recorded_livelock_regression
cargo test -p lisa-core
cargo clippy -p lisa-core --all-targets -- -D warnings
git diff --check -- crates/lisa-core/tests/recorded_livelock_regression.rs
```

Resolve any test-owned failure before committing.

## Step 11: run workspace regression verification

Run `cargo test --workspace`. Investigate every actual failure. Existing
environment-gated ignored tests may remain ignored according to their existing
contract.

This ticket does not claim T-041-02-03's final release WASM size-budget gate,
but the source-only integration test cannot enter a production WASM artifact.

## Step 12: write implementation progress

Create attempt-private `progress.md` recording completed work, exact checks,
test counts where available, deviations, and remaining Review work.

Do not publish directly to `docs/active/work/T-041-02-01`.

## Step 13: commit the exact source unit

Run:

```text
lisa commit-ticket --ticket-id T-041-02-01 \
  --message "test(core): replay recorded completion livelock" \
  --include crates/lisa-core/tests/recorded_livelock_regression.rs
```

Do not use the ordinary index. Inspect status and the commit afterward to prove
the exact file was committed and unrelated orchestration changes remain
untouched.

## Step 14: complete Review

Write attempt-private `review.md` and a valid `review-disposition.json`. Pass
only if the source path is committed, all required checks are green, and the
acceptance assertions are present. Otherwise block with a specific actionable
reason.

Remain on T-041-02-01 after Review and wait for Lisa's completion commit.

