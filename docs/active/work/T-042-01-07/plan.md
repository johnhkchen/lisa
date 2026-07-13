# Plan: suppress false Review timeout

## Goal

Turn the existing Review-admission timeout guard into a complete regression
contract across missing evidence and completion transaction states.

The implementation should be one atomic plugin source unit.

## Step 1: make native launch-error behavior selectable

Modify the test-only portion of `State` in
`crates/lisa-plugin/src/lib.rs`.

Add a boolean that requests production command-builder failure handling in a
selected native test.

Keep the default false so existing tests preserve their inert executor.

Update the command-builder error branch in `execute_completion_effect`.

When compiling tests and the flag is false, retain the existing stub behavior.

Otherwise run the existing production cleanup and rejection logic.

Verification:

- `cargo check -p lisa-plugin --tests` compiles;
- existing adapter tests still see pending effects with default State;
- a strict fixture can observe false dispatch and LaunchFailed activity.

## Step 2: add authoritative missing-Review coverage

Add a native unit test with a scanned Review ticket, Running thread, current
attempt lease, private attempt directory, and expired clocks.

Do not create `review.md`.

Call `check_review_timeouts`.

Assert exactly one prompt marker/event.

Call it again and assert idempotence if useful.

This differs from the historical timeout test by proving absence through an
actual current lease.

Verification:

- focused test passes;
- adding a private Review would make the prompt assertion fail.

## Step 3: add admitted pending and confirmed coverage

Construct valid Git-root/nested-project paths.

Create Review and passing disposition in the exact private attempt directory.

Dispatch `CompletionInput::Reconcile`.

Assert:

- one launch effect was recorded;
- pending completion belongs to the current lease;
- timeout sends no prompt and does not add a marker.

For confirmed coverage, update the ticket to durable Done, rebuild the DAG,
remove pending state as needed to model confirmed reconstruction, and leave the
thread present long enough to call timeout handling.

Assert timeout still sends no prompt because exact Review admission remains
true.

Verification:

- pending state is unchanged by timeout suppression;
- durable Done is not converted into a generic pane prompt.

## Step 4: add nested-path launch rejection coverage

Build a temporary topology with:

- Git root;
- nested Lisa project `games/midsummer`;
- work path below the nested project;
- scanned ticket path outside the Git root.

Configure dummy `lisa_bin`, project root, Git root, and strict native
launch-error behavior.

Install the exact lease and private passing Review.

Dispatch Reconcile through the typed adapter.

Assert:

- builder rejects the outside-root ticket path;
- dispatch returns false;
- pending state is cleaned up;
- LaunchFailed activity retains expected correlation and path detail;
- activity-to-UI conversion retains kind, ticket, correlation, and detail;
- timeout remains silent because Review is admitted.

Verification:

- the test would fail under the current unconditional native error bypass;
- the test would fail if timeout ignores admitted Review after rejection.

## Step 5: add retryable command-result failure coverage

Build the valid counterpart topology where ticket and work paths are inside the
same Git root and project is nested.

Configure dummy `lisa_bin` so command construction succeeds.

Install an exact passing Review and dispatch Reconcile.

Feed a nonzero completion result with recognizable stderr.

Assert:

- pending state is removed;
- thread, slot if present, and current lease remain;
- structured LaunchFailed activity contains recoverable retry text;
- correlation matches the current attempt generation;
- UI conversion preserves the structured rejection;
- timeout emits no prompt.

Optionally reconcile once more after the timeout assertion and check a second
effect/pending record, proving the adapter can retry from durable evidence.

Verification:

- test fails if result failure releases the seat or hides rejection;
- test fails if artifact-based timeout suppression is replaced by pending-only
  suppression.

## Step 6: refactor test setup only if duplication obscures behavior

Prefer existing helpers:

- `install_current_attempt`;
- `write_passing_review_disposition`;
- `CompletionGenerationId` constructors;
- `activity_event_to_ui_entry`.

Add narrowly named helpers for aged Review state or rejection lookup only when
they reduce repeated mechanics.

Do not abstract the scenario-specific state transitions.

Verification:

- each test reads as a chronological incident trace;
- helper names retain lease and artifact semantics.

## Step 7: format and run focused tests

Run:

```text
cargo fmt --all
cargo test -p lisa-plugin --lib review_timeout_ --no-fail-fast
```

If the name filter includes unrelated historical tests, record the exact test
count and ensure all pass.

Run any nested completion test separately if its name does not include the
common prefix.

Verification:

- all new tests pass;
- no unexpected files are created outside known pre-existing test output;
- `cargo fmt --all -- --check` passes.

## Step 8: run package and workspace verification

Run:

```text
cargo test -p lisa-plugin --lib --no-fail-fast
cargo test --workspace --no-fail-fast
```

The full plugin suite is mandatory because the executor test seam changes
default error handling selection.

The workspace suite is mandatory because plugin tests link CLI test support
and consume core completion types.

Verification:

- no plugin regression;
- no CLI/core integration regression;
- the environment-gated real-Zellij test may remain ignored as declared.

## Step 9: run lint and build gates

Run:

```text
cargo clippy -p lisa-plugin --all-targets -- -D warnings
cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings
cargo build -p lisa-plugin --target wasm32-wasip1 --release
cargo fmt --all -- --check
git diff --check -- crates/lisa-plugin/src/lib.rs
```

Native all-target Clippy checks the test-only field and regression code.

WASM Clippy and build check the real production compilation shape.

Verification:

- zero warnings under denied-warnings policy;
- release WASM artifact builds;
- source diff is whitespace-clean.

## Step 10: inspect the source diff

Review the entire diff for `crates/lisa-plugin/src/lib.rs`.

Confirm:

- no new completion launch gateway;
- no production timeout behavior regression;
- no core reducer duplication;
- strict launch-error control is test-only;
- correlations derive from exact lease identity;
- tests assert structured rejection and UI conversion;
- no unrelated formatting churn.

Run a source search confirming `execute_completion_effect` remains the sole
completion executor call site.

## Step 11: write progress and commit the meaningful source unit

Update attempt-private `progress.md` before source commit with:

- completed implementation steps;
- focused and broad verification results;
- deviations from this plan;
- remaining review work.

Commit only the plugin source through Lisa:

```text
lisa commit-ticket \
  --ticket-id T-042-01-07 \
  --message "test(plugin): cover Review timeout completion states" \
  --include crates/lisa-plugin/src/lib.rs
```

Do not use `git add` or ordinary `git commit`.

Verification:

- command returns a commit ID;
- `git diff-tree --no-commit-id --name-only -r <commit>` lists only the plugin
  source file;
- source file is no longer modified;
- ordinary index remains empty.

## Step 12: final repository and acceptance audit

Check worktree state without altering unrelated files.

Distinguish Lisa-managed and other-ticket changes from ticket-owned source.

Map acceptance explicitly:

- missing Review -> prompt;
- admitted pending Review -> no prompt;
- confirmed Review -> no prompt;
- nested-path launch rejection -> correlated rendered rejection, no prompt;
- retryable command failure -> correlated rendered rejection, no prompt and
  retry authority retained.

Confirm phase/status frontmatter was not manually changed.

## Step 13: write Review artifacts

Write attempt-private `review.md` summarizing:

- source commit and exact path;
- implementation shape;
- scenario coverage;
- verification results;
- repository preservation;
- limitations and open concerns.

Write exactly:

```json
{"disposition":"pass","reason":null}
```

when all gates pass and no blocker remains.

Use block disposition only for a concrete actionable unresolved failure.

After both artifacts exist, remain on `T-042-01-07` and stop.

Do not publish Done, edit ticket frontmatter, release the seat, or begin another
ticket.
