# Progress: recorded Review livelock regression

## Status

Implementation is complete. The deterministic T-009-01-01 trace is encoded in
a new lisa-core integration test, all planned verification is green, and the
exact source unit is committed through Lisa's isolated transaction. Only Review
remains.

## Completed work

### RDSPI discovery and design

- Read `CLAUDE.md`, the ticket, and the complete RDSPI workflow.
- Read parent story S-041-02 and its explicit test-only boundary.
- Read the staged Review-completion field note.
- Read the settled completion domain implementation and predecessor artifacts.
- Confirmed production completion code must remain unchanged.
- Selected a standalone public integration test as the disjoint ownership unit.
- Wrote Research, Design, Structure, and Plan artifacts to the private attempt.

### Test source

Created:

```text
crates/lisa-core/tests/recorded_livelock_regression.rs
```

No production source, Cargo manifest, lockfile, plugin, or CLI file changed.

### Recorded event fixture

The test defines the exact ordered milestones:

1. Review artifact written;
2. phase advanced to Review;
3. stop observed;
4. Review timeout elapsed at 600 seconds;
5. reload observed;
6. manual completion result confirmed.

The artifact-before-phase order is the crucial historical edge.

Stable opaque identities attribute the fixture to:

- attempt `T-009-01-01/attempt-1`;
- completion `T-009-01-01/completion-1`;
- command correlation `T-009-01-01/manual-result`.

### Aggregate-backed replay

The fixture driver retains only the adapter-side facts absent from the pure
contract: whether phase Review has been observed and whether the Review
artifact is present.

All lifecycle decisions use production public APIs:

- `reconcile` re-derives the completion obligation;
- `reduce` applies Request;
- `reduce` applies CommandLaunched;
- `reduce` applies matching CommandSucceeded.

The driver verifies the reconciliation effect and reducer effect are identical.
It verifies the launch correlation is retained through stop, timeout, and
reload observations. It verifies Confirmed reconciliation emits no request.

### Exact result assertions

The aggregate replay asserts:

- exactly 1 completion Request;
- exactly 1 authoritative Confirmed transition;
- exactly 0 finish-up prompts;
- exactly 0 re-requests;
- final state `CompletionState::Confirmed`.

Finish-up is recorded as synthetic fixture adapter output because the pure core
contract intentionally has no pane-prompt effect. The timeout policy suppresses
that output while the Review artifact fact is present.

### Naive edge-triggered negative control

The same event slice also drives a deliberately naive stub. That model requests
only when the artifact-created event arrives after phase Review and never
revisits the durable artifact on the later phase edge.

The regression asserts the exact naive failure:

- 0 aggregate requests;
- 1 later manual confirmation;
- 1 stale finish-up prompt;
- 0 re-requests.

This executable counterexample proves the fixture distinguishes the settled
level-triggered contract from the historical edge-triggered mistake.

## Verification results

### Formatting

```text
cargo fmt --all
cargo fmt --all -- --check
```

Passed.

### Focused integration test

```text
cargo test -p lisa-core --test recorded_livelock_regression
```

Passed: 1 test, 0 failed.

### Core regression suite

```text
cargo test -p lisa-core
```

Passed:

- 191 unit tests;
- 1 integration test;
- 0 doctest failures.

### Lint

```text
cargo clippy -p lisa-core --all-targets -- -D warnings
```

Passed with warnings denied.

### Diff hygiene

```text
git diff --check -- crates/lisa-core/tests/recorded_livelock_regression.rs
```

Passed.

### Workspace regression suite

```text
cargo test --workspace
```

Passed with zero failures. Observed principal suites:

- lisa-cli unit tests: 279 passed;
- CLI integration suites: 1 + 3 + 1 passed;
- lisa-core unit tests: 191 passed;
- recorded regression integration: 1 passed;
- lisa-plugin unit tests: 341 passed;
- core doctests: 0 failed;
- real-Zellij integration: 1 existing environment-gated test ignored.

## Plan deviations

No functional deviation.

The final release WASM build/size budget was not run because ticket
T-041-02-03 explicitly owns that settled-tree barrier after both deterministic
and generated suites land. This ticket adds only a native integration test and
no production dependency.

## Ownership and repository preservation

The ticket-owned source set is exactly:

```text
crates/lisa-core/tests/recorded_livelock_regression.rs
```

The ordinary Git index is empty. Lisa/orchestration changes to active tickets,
provenance, and published phase artifacts remain outside source ownership.
Untracked plugin documentation is unrelated and untouched.

## Isolated source commit

The exact command completed successfully:

```text
lisa commit-ticket --ticket-id T-041-02-01 \
  --message "test(core): replay recorded completion livelock" \
  --include crates/lisa-core/tests/recorded_livelock_regression.rs
```

Commit:

```text
e28d71209ae5cb2722894c96e29f596e3d7df7a9
test(core): replay recorded completion livelock
```

Post-commit inspection confirms the commit contains exactly the integration
test path. The ticket-owned source path is clean and the ordinary index remains
empty.

## Remaining steps

1. Write Review and the pass/block disposition.
2. Remain on T-041-02-01 for Lisa's completion commit.

