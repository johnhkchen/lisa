# Progress: bounded reconciliation replay convergence

## Status

Implementation source is committed; final verification is in progress.

The core deadline unit is complete, tested, and committed.

The plugin journal, replay adapter, terminal timeout transition, and acceptance
regressions are implemented and committed, awaiting final verification against
a concurrent plugin UI change.

## Completed: durable core deadline

Added `CompletionDeadline` to `lisa-core` as an opaque Unix epoch millisecond
value.

The type supports durable construction/access and inclusive expiration checks.

Extended CommandInFlight with its exact absolute deadline while retaining its
mandatory CorrelationId.

Extended CommandLaunched so the reducer creates both values atomically in typed
state.

Changed pure reconciliation to accept explicit current time.

Before deadline it returns `ReplayCommandInFlight` with exact identity.

At and after deadline it returns `CommandInFlightDeadlineExceeded`.

Eligible, Requested, Rejected, Confirmed, correlation mismatch, and effect
semantics remain unchanged outside this time-aware in-flight branch.

Updated in-module core tests for inclusive boundaries and state retention.

Updated both external core regressions with deterministic current/deadline
values.

The generated event-ordering property still observes exactly one live effect.

The recorded livelock regression still confirms once with no stale finish-up or
re-request.

Verification completed:

```text
cargo test -p lisa-core
196 unit tests passed
completion_state_machine passed
recorded_livelock_regression passed
doc tests passed
```

Committed through Lisa's isolated transaction:

```text
325b1bea973e2069c98d1878e25d45ac7af5de1d
feat(core): bound completion reconciliation
```

Exact committed paths:

- `crates/lisa-core/src/completion.rs`
- `crates/lisa-core/tests/completion_state_machine.rs`
- `crates/lisa-core/tests/recorded_livelock_regression.rs`

## Completed: journal deadline persistence

Extended the typed CommandInFlight journal transition with
`CompletionDeadline`.

New JSON records write `reconciliation_deadline_unix_ms`.

The field is optional on read for compatibility with schema-1 histories written
by T-042-02-02.

A legacy missing field maps to deadline zero, causing the formerly unbounded
state to expire action-required on reconciliation.

Kept schema version 1 because this is an additive compatible record field.

The journal folds correlation and deadline through the core reducer.

Action-required Rejected now masks uncertain Done bytes.

Retryable Rejected remains unmasked and can begin a new request.

Added tests for exact deadline serialization/reconstruction, legacy missing
field behavior, retryable masking behavior, and action-required masking.

The journal suite passed as part of the first full plugin run before the later
acceptance regressions were added.

## Completed: initial bounded launch

Added a named 60-second completion reconciliation timeout next to scheduler poll
timing.

Added deterministic conversion from `SystemTime` to the durable deadline type.

The production dispatcher still uses `SystemTime::now()`.

Tests can call an internal explicit-time dispatcher without sleeping.

The initial executor computes one deadline and persists it with CommandInFlight
before the host call.

Live pending state retains the same deadline and identifies whether it is an
initial invocation or reconciliation replay.

## Completed: exact-key replay

Reconcile before deadline calls a focused replay adapter.

The adapter verifies:

- no host command is already pending;
- the source lease is still current;
- journal key attempt matches the lease;
- journal correlation and deadline match the pure decision;
- the ticket path remains available.

It rebuilds `complete-ticket` argv from the original durable generation key.

It installs a live replay pending entry before crossing the host boundary.

It does not append duplicate Requested or CommandInFlight records.

Initial and replay launches share one physical host-command method, retaining
the project's one-launch-boundary characterization.

Duplicate stop and repeated poll reconciliation cannot create another command
while the replay is pending.

## Completed: bounded replay failure and expiry

A failed initial host command retains existing retryable Rejected behavior.

A failed reconciliation replay does not prove the original command failed.

It therefore removes only live replay pending state, logs the failure, and
retains the original CommandInFlight deadline.

Subsequent replay attempts cannot reset that absolute window.

At the deadline, the plugin revalidates exact in-flight identity and atomically
appends correlated Rejected with `Retryability::ActionRequired`.

Only after the durable append does it remove pending state and rebuild the DAG.

Further reconciliation returns no effect.

Late results have no pending entry and cannot override the terminal state.

Uncertain Done bytes stay masked to prior Review state.

## Completed: acceptance regressions

Added a real temporary-Git lost-result convergence test.

The test:

1. drives the production adapter to durable Requested and CommandInFlight;
2. executes the adapter's key through `lisa_cli::complete_ticket`;
3. deliberately loses the successful plugin result;
4. reconstructs a fresh plugin from the journal;
5. presents duplicate Stop before reconciliation;
6. launches one same-key replay before deadline;
7. proves repeated Stop/Reconcile observations do not duplicate it;
8. executes the CLI transaction again;
9. asserts the CLI returns the original commit with no new commit;
10. delivers the replay result and asserts one Confirmed record, one
    authoritative Done provenance record, and scheduler release.

Added a deterministic exact-deadline regression.

It writes Done bytes while in-flight, reconciles at the inclusive deadline,
asserts action-required Rejected, verifies Review masking, then presents later
Reconcile and duplicate Stop observations and proves effect/journal counts do
not increase.

## Verification completed before concurrent overlap

Before the two final acceptance tests were inserted:

```text
cargo test -p lisa-plugin
359 passed; 1 failed
```

The sole failure was the source characterization counting two physical host
launch sites.

That was corrected by extracting one shared launch method.

The focused characterization then passed:

```text
cargo test -p lisa-plugin completion_has_one_typed_request_gateway
1 passed
```

`cargo fmt --all` was run after the implementation and new tests.

## Concurrent worktree event

T-042-03-02 is concurrently modifying `crates/lisa-plugin/src/ui.rs` and
`crates/lisa-plugin/src/lib.rs` for operator modal confirmation.

During the acceptance-test rebuild it added `ModalState::operator_outcome` in
`ui.rs` before completing the corresponding `lib.rs` fixture/adapter update.

The resulting compiler error is outside this ticket's ownership:

```text
missing field `operator_outcome` in initializer of `ui::ModalState`
```

This ticket has not patched or reverted that unrelated field.

The concurrent active ticket and its source are being preserved.

## Plugin source commit

Because both active tickets modify `crates/lisa-plugin/src/lib.rs`, leaving this
ticket's complete unit uncommitted risked the concurrent ticket absorbing it in
its exact-path transaction.

The journal and plugin unit were therefore committed while the unrelated UI
transition remained uncommitted:

```text
9f09baba88db1438645b8a3d292eb7035ae35b4f
fix(plugin): converge bounded completion replay
```

Exact committed paths:

- `crates/lisa-plugin/src/completion_journal.rs`
- `crates/lisa-plugin/src/lib.rs`

The commit contains no `ui.rs`, ticket, provenance, or shared work artifact.

If final acceptance verification finds a ticket-owned defect after the UI API
becomes consistent, it will be fixed in a follow-up exact-path transaction.

## Remaining

- await T-042-03-02's fix/commit for its failing modal wrapping assertion;
- rerun the full plugin and workspace suites;
- run `just check`;
- audit ticket-owned paths clean;
- complete Review artifacts and disposition.

## Deviations from plan

The core unit was committed while the plugin unit remained in progress because
the shared worktree acquired an unrelated transient compile error.

The plugin unit was then committed before final test execution to protect the
shared `lib.rs` boundary from a concurrent same-file commit.

This preserves the plan's independent meaningful unit and avoids absorbing the
concurrent ticket's changes.

No ticket-owned source was staged through the ordinary index.

## Acceptance verification after API consistency

After T-042-03-02 completed the missing struct initializers, both ticket-owned
focused regressions passed:

```text
cargo test -p lisa-plugin \
  lost_result_reload_duplicate_stop_replay_converges_on_single_prior_commit
1 passed

cargo test -p lisa-plugin \
  reconciliation_deadline_ends_action_required_without_infinite_replay
1 passed
```

The first test exercised real `lisa_cli::complete_ticket` twice with one key.

The second result returned the first commit, reported no committed paths, and
left Git at exactly base plus one completion commit.

The plugin then wrote exactly one Confirmed journal record and one authoritative
Done provenance record.

The deadline regression ended in action-required Rejected at the inclusive
boundary and kept launch/journal counts unchanged under later Reconcile and
Stopped inputs.

CLI verification also completed:

```text
cargo test -p lisa-cli
14 library tests passed
267 binary tests passed
atomic provider contract passed
help surface passed
preownership status passed
real Zellij boundary remained intentionally ignored
```

The first broad plugin run after the concurrent API became compilable executed
all 363 tests.

All 361 pre-existing/non-modal tests and both new replay tests passed.

The only failure belongs to concurrent T-042-03-02:

```text
ui::tests::operator_modal_outcomes_render_ticket_correlation_and_named_reason
assertion failed: rendered.contains(&detail)
```

That test and `ui.rs` were introduced after this ticket's source commits.

This ticket has not modified or weakened it.

`cargo fmt --all -- --check` and `git diff --check` both pass on the combined
worktree.

## Final verification

T-042-03-02 corrected its transient modal assertion before the final gates.

The complete plugin suite then passed:

```text
cargo test -p lisa-plugin
364 passed; 0 failed
```

The complete workspace suite passed after a fresh combined build:

```text
cargo test --workspace
all lisa-core, lisa-cli, lisa-plugin, integration, property, and doc tests passed
the explicitly environment-dependent real Zellij test remained ignored
```

The project check passed:

```text
just check
cargo check -p lisa-plugin --target wasm32-wasip1: passed
cargo test --workspace: passed
```

Formatting and diff integrity passed:

```text
cargo fmt --all -- --check
git diff --check
```

## Final source ownership

Both ticket source commits contain exact ticket-owned paths only.

The ordinary index has no staged paths.

The core files and `completion_journal.rs` are clean.

`crates/lisa-plugin/src/lib.rs` has later uncommitted changes from concurrent
T-042-03-02 only; its diff begins with that ticket's `OperatorModalOutcome` and
does not alter the committed replay unit.

The concurrent `ui.rs`, active ticket frontmatter, provenance ledger, canonical
work artifacts, and unrelated untracked plugin docs were not included in this
ticket's commits.

No implementation step used ordinary `git add` or ordinary `git commit`.

## Implement result

All planned ticket-owned implementation and verification work is complete.

No follow-up source correction was required after the two commits.

The ticket is ready for Review disposition pass.
