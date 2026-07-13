# Progress: suppress false Review timeout

## Status

Implementation and verification are complete.

Research, Design, Structure, and Plan are complete in the attempt-private work
directory.

The repository already contains the production timeout-admission guard from
dependency `T-042-01-03`.

This ticket's source unit is composed regression coverage plus a narrow native
test seam for production command-builder failure handling.

## Completed

- Read repository agent instructions and the complete assignment.
- Read the ticket, story, workflow, dependency tickets, dependency reviews,
  plugin timeout path, completion adapter, command builder, result handler,
  deadline evaluator, core reducer/reconciler, UI rendering, and related tests.
- Identified the four explicit acceptance scenarios and the additional
  confirmed-state clause.
- Confirmed the existing guard re-admits exact-current-attempt `review.md`
  before pane I/O.
- Confirmed launch/result rejection activity already carries typed kind,
  completion-generation correlation, and detail into UI state.
- Identified the native-test gap: command construction failures are normally
  stubbed as successful pending launches.
- Selected a one-file plugin change with no core, CLI, manifest, lockfile, or
  production-state-machine expansion.
- Wrote `research.md`, `design.md`, `structure.md`, and `plan.md`.

## Concurrent ownership observation

While beginning the source edit, another active Lisa ticket,
`T-042-02-02`, modified `crates/lisa-plugin/src/lib.rs` and created
`crates/lisa-plugin/src/completion_journal.rs`.

That ticket is implementing the later durable completion journal story.

Its source changes are not owned by `T-042-01-07`.

Because this ticket must eventually commit `crates/lisa-plugin/src/lib.rs`
through an exact Lisa isolated transaction, applying and committing while the
other ticket's changes remain uncommitted would incorrectly capture both
tickets' work.

The concurrent ticket committed its exact source unit as `5e6df88` before this
ticket applied any overlapping patch.

This ticket then implemented only test helpers and acceptance regressions in
`crates/lisa-plugin/src/lib.rs` on top of that clean commit.

No ordinary-index command has been used.

The implementation will resume on top of the other ticket's committed state,
or will be adjusted to avoid overlap if that ticket changes the relevant
boundary materially.

## Implemented source unit

Added shared native fixture helpers that:

- scan real Review tickets;
- install exact current attempt leases;
- age Review/activity clocks beyond timeout policy;
- create attempt-private passing Review evidence;
- assert timeout silence;
- locate exact correlated LaunchFailed activity;
- prove activity-to-UI conversion preserves all rejection fields.

Added four acceptance regressions:

1. `review_timeout_prompts_only_when_current_attempt_review_is_missing`
   proves an expired Review attempt with a current lease and no private
   `review.md` receives exactly one finish-up event.
2. `review_timeout_suppresses_admitted_pending_and_confirmed_completion`
   drives the typed Reconcile adapter into durable CommandInFlight, re-runs
   reconciliation to surface its action-required correlation without a second
   effect, proves timeout silence, then confirms durable Done and proves the
   completed state emits no prompt.
3. `review_timeout_preserves_nested_path_launch_rejection` places a scanned
   ticket outside the Git root while the Lisa project remains nested at
   `games/midsummer`; the real command builder rejects it, pending state is not
   created, exact correlated LaunchFailed activity renders unchanged, and the
   admitted Review suppresses the timeout prompt.
4. `review_timeout_preserves_retryable_command_failure` drives a valid nested
   command into pending state, supplies a nonzero command result, proves the
   durable aggregate is Rejected/Retryable, exact lease and thread remain,
   correlated failure renders unchanged, and timeout emits no prompt.

No production timeout, adapter, reducer, journal, CLI, or UI implementation was
changed by this ticket.

## Verification results

Focused timeout filter:

```text
cargo test -p lisa-plugin --lib review_timeout_ --no-fail-fast
```

Passed: 6; failed: 0.

The filter includes the four new regressions and two existing timeout tests.

Full plugin library suite:

```text
cargo test -p lisa-plugin --lib --no-fail-fast
```

Passed: 358; failed: 0.

Full workspace suite:

```text
cargo test --workspace --no-fail-fast
```

Passed across CLI library, CLI binary, CLI integration tests, core unit and
integration tests, plugin library, and doctests. No failure occurred. The
existing environment-gated real-Zellij test remained ignored under its normal
contract.

Lint, formatting, build, and hygiene:

```text
cargo clippy -p lisa-plugin --all-targets -- -D warnings
cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings
cargo build -p lisa-plugin --target wasm32-wasip1 --release
cargo fmt --all -- --check
git diff --check
```

All passed.

The release WASM build completed successfully.

Source inspection confirms there remains exactly one production call to
`self.execute_completion_effect`.

The source diff is 358 inserted test lines in one file with no production-line
change.

## Source commit

Committed through Lisa's isolated transaction:

```text
ec6ae053c9b31b30bd451d2cc8378d9e22f04a0a
test(plugin): cover Review timeout completion states
```

Exact include:

```text
crates/lisa-plugin/src/lib.rs
```

`git diff-tree` lists exactly that one path.

The file is clean after the commit.

The ordinary Git index is empty.

Lisa-managed provenance/ticket/workflow changes, unrelated ticket artifacts,
and pre-existing plugin test outputs remain outside the source commit.

## Remaining

- Write `review.md` and `review-disposition.json`.

## Deviations

The Plan assumed the plugin adapter state present at the start of this attempt.

The concurrent durable-journal ticket is changing that state representation
while this ticket is active.

The implementation followed that adjustment: it uses the newly committed
durable aggregate and its non-empty-journal native launch-error behavior.

The planned new test-only State boolean was unnecessary and was not added.

The source unit is regression-only because dependency `T-042-01-03` had already
implemented the production admission guard required by this ticket.
