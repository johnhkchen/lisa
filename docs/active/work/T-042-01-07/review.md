# Review: suppress false Review timeout

## Disposition

Pass.

`T-042-01-07` now has plugin-native regression coverage for every named Review
timeout scenario in the ticket.

The generic finish-up prompt is proven to occur only when the exact current
attempt lacks `review.md`.

An admitted Review suppresses the prompt while completion is in flight, action
is required on an in-flight correlation, durable confirmation exists, nested
path command construction is rejected, or a command result is retryable.

Correlated rejection evidence remains structured and reaches the UI adapter
unchanged.

All focused, plugin, workspace, lint, formatting, hygiene, and release WASM
gates pass.

No blocking issue remains.

## Source commit

The meaningful source unit is:

```text
ec6ae053c9b31b30bd451d2cc8378d9e22f04a0a
test(plugin): cover Review timeout completion states
```

It was created through:

```text
lisa commit-ticket \
  --ticket-id T-042-01-07 \
  --message "test(plugin): cover Review timeout completion states" \
  --include crates/lisa-plugin/src/lib.rs
```

`git diff-tree` confirms the commit contains exactly:

- `crates/lisa-plugin/src/lib.rs`.

No production file was created or deleted.

No core, CLI, UI, journal, manifest, lockfile, ticket, provenance, shared work
artifact, or unrelated source path was included.

## Production behavior under review

Dependency `T-042-01-03` had already introduced the production guard used by
this ticket.

`check_review_timeouts` first selects time-eligible Running Review threads.

Before resolving an adapter or writing to the pane, it calls
`review_completion_suppresses_finish_up`.

That helper suppresses immediately for a live pending completion.

Otherwise it obtains the thread's exact attempt lease, verifies current
authority, and re-admits private `review.md` through the same boundary used by
completion reconciliation.

`Ok(true)` suppresses the generic prompt.

`Ok(false)` allows it.

Admission error logs actionable Error activity and suppresses the misleading
prompt.

This ticket did not duplicate or alter that production policy.

It proves the guard composes with the completed effect adapter, nested path
normalization, correlated rendering, and durable aggregate journal.

## Files changed

### `crates/lisa-plugin/src/lib.rs`

Added test-only helper functions for:

- constructing an expired Review attempt from real scanned ticket paths;
- installing an exact current attempt lease;
- selecting strict native launch-error handling through a real journal path;
- writing private Review and Pass disposition evidence;
- asserting absence of finish-up markers/events;
- locating an exact correlated LaunchFailed event;
- verifying activity-to-UI rejection conversion field by field.

Added four acceptance regressions.

The source diff contains 358 inserted test lines and zero production-line
changes.

## Missing Review regression

`review_timeout_prompts_only_when_current_attempt_review_is_missing` creates a
real scanned Review ticket in a nested project.

It installs a Running Review thread and exact current lease.

Both phase and activity clocks are older than the timeout bars.

The attempt-private directory exists, but `review.md` does not.

The test calls the real `check_review_timeouts` method.

It observes exactly one FinishUpPromptSent event for pane 42 and the matching
idempotence marker.

This is stronger than the historical timeout characterization, which had no
current attempt lease and therefore did not exercise authoritative absence.

## Pending, in-flight, action-required, and confirmed regression

`review_timeout_suppresses_admitted_pending_and_confirmed_completion` uses a
Git root containing a Lisa project at `games/midsummer`.

It writes current-attempt `review.md` and a passing disposition.

Typed `CompletionInput::Reconcile` admits the evidence and reaches the sole
effect executor.

The newly durable completion journal records CommandInFlight with the exact
completion-generation correlation.

The pending map holds the same attempt authority.

The test re-runs level-triggered reconciliation while in flight.

That produces no second effect and logs the existing action-required warning
with the exact correlation.

Review timeout then emits neither a prompt event nor a sent marker.

Pending state remains intact.

The fixture next writes durable Done and supplies a successful correlated
command result.

The aggregate becomes Confirmed and the completed thread is removed.

A later timeout check remains silent.

This proves the timeout never substitutes missing-artifact advice for pending,
in-flight/action-required, or confirmed completion truth.

## Nested path launch rejection regression

`review_timeout_preserves_nested_path_launch_rejection` creates:

- a Git root;
- a nested Lisa project at `games/midsummer`;
- a valid nested work directory;
- a scanned ticket path outside the Git root.

The fixture configures a non-empty durable journal path, which selects the
production command-builder failure path in native tests.

Current-attempt Review and Pass disposition are admitted first.

The typed Reconcile request then reaches the real command builder.

Path normalization rejects the ticket because it is outside the Git root.

Dispatch returns false.

No pending record or aggregate is fabricated.

The activity log retains LaunchFailed with:

- the ticket ID;
- distinct rejection kind;
- exact completion-generation correlation;
- actionable outside-Git-root detail.

`activity_event_to_ui_entry` preserves all four fields unchanged.

Despite the old timeout clocks and absence of pending state, the admitted
Review suppresses the finish-up prompt.

This is the critical field shape: completion failed, but Review already exists.

## Retryable command failure regression

`review_timeout_preserves_retryable_command_failure` uses valid nested ticket
and work paths.

Reconcile reaches CommandInFlight and creates pending state.

The test supplies a nonzero command result with recognizable nested identity
failure detail.

The result handler durably records Rejected with
`Retryability::Retryable`.

Pending state is removed, while the thread and exact current lease remain.

The structured LaunchFailed activity includes both the command diagnostic and
the existing “recoverable for retry” operator message.

Its completion-generation correlation renders unchanged through the UI
adapter.

With no pending map entry left, timeout still emits no prompt because the exact
Review is admitted.

The adapter's reconciliation state remains explicitly Rejected/Retryable.

This distinguishes the actual completion failure from missing Review work.

## Acceptance mapping

### Prompt only when Review is genuinely absent

Satisfied by the authoritative missing-Review test.

### Admitted Review pending

Satisfied by the CommandInFlight/pending portion of the composed test.

### Confirmed completion

Satisfied by the durable Done plus successful result portion.

### Action-required state

Satisfied by repeat reconciliation of the durable in-flight aggregate, which
emits its correlation-bearing action-required warning and no duplicate effect
or timeout prompt.

### Nested-path launch rejection

Satisfied through the real path mapper/builder and typed dispatcher.

### Retryable command failure

Satisfied through the real command-result handler and durable Rejected state.

### Correlated state remains visible

Satisfied through structured activity assertions and UI conversion assertions
for both failure scenarios, plus the in-flight action-required warning.

## Verification results

Focused timeout tests:

```text
cargo test -p lisa-plugin --lib review_timeout_ --no-fail-fast
```

Passed: 6; failed: 0.

The filter covers the four new regressions and two existing timeout tests.

Full plugin library:

```text
cargo test -p lisa-plugin --lib --no-fail-fast
```

Passed: 358; failed: 0.

Full workspace:

```text
cargo test --workspace --no-fail-fast
```

Passed across CLI library/binary/integration, core unit/integration, plugin
library, and doctest targets.

No test failed.

The existing real-Zellij environment-gated test retained its declared ignored
status.

Native lint:

```text
cargo clippy -p lisa-plugin --all-targets -- -D warnings
```

Passed.

WASM lint:

```text
cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings
```

Passed.

Release WASM build:

```text
cargo build -p lisa-plugin --target wasm32-wasip1 --release
```

Passed.

Formatting and hygiene:

```text
cargo fmt --all -- --check
git diff --check
```

Both passed.

The structural source search still finds exactly one production invocation of
`self.execute_completion_effect`.

## Repository preservation

The ordinary Git index is empty.

The ticket-owned source file is clean after the isolated commit.

Lisa-managed `.lisa/provenance.jsonl` and active ticket changes were not
included.

Attempt artifacts were not committed as source; Lisa owns their admission and
final publication.

Unrelated `T-042-02-02` workflow artifacts remain untouched.

Pre-existing untracked plugin test artifacts remain untouched.

No ordinary `git add`, broad add, or ordinary `git commit` was used.

## Concurrent ownership observation

During Implement, `T-042-02-02` was actively changing the same plugin file for
the durable completion journal.

This is a missing cross-story dependency/ownership edge in the active DAG.

This attempt did not patch or commit while those edits were uncommitted.

It waited until that ticket landed its isolated source commit `5e6df88`, then
re-read the settled adapter and applied this ticket's test-only diff on top.

The final commit therefore contains no journal-ticket changes.

The durable journal improved this ticket's regression fidelity by making
CommandInFlight, Rejected/Retryable, and Confirmed states directly observable.

The initially planned test-only State flag was unnecessary and was not added.

## Open concerns and limitations

No blocking concern exists.

The source change is regression-only because the declared predecessor already
implemented the production admission guard.

That is intentional convergence work: this ticket proves the guard against the
real adapter outcomes that previously hid Arcade completion failures.

The nested launch rejection occurs before a journal Requested transition, so
its state is represented by correlated activity rather than a durable rejected
aggregate. The activity is the correct available operator surface for this
pre-launch configuration error, and the test proves it is not hidden by a pane
prompt.

The tests use the native Zellij host shim; they do not launch a real
`complete-ticket` subprocess. The command builder, typed adapter, journal,
result handler, and UI conversion are real. The connected real transaction
boundary remains covered by dependency `T-042-01-06`.

The missing DAG edge between tickets touching `lib.rs` should be considered in
future decomposition, but it does not affect the correctness or ownership of
the final commits.

## Critical issues requiring human attention

None.

Review is complete. This attempt remains on `T-042-01-07` for Lisa to admit the
Review, prepare and verify the completion commit, publish Done, and release the
seat.
