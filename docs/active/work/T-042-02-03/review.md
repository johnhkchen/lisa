# Review: bounded reconciliation replay convergence

## Disposition

Pass.

The acceptance criterion is met.

An unresolved completion now has durable command identity and a durable absolute
deadline.

A reconstructed command replays the exact same generation through the existing
idempotent CLI transaction.

A successful replay converges on the prior completion commit and publishes one
authoritative Done result.

An unconfirmable command ends in a named action-required rejection at its
inclusive deadline.

No automatic replay continues after that state.

## Source commits

Core domain and regressions:

```text
325b1bea973e2069c98d1878e25d45ac7af5de1d
feat(core): bound completion reconciliation
```

Plugin journal, adapter, and acceptance regressions:

```text
9f09baba88db1438645b8a3d292eb7035ae35b4f
fix(plugin): converge bounded completion replay
```

Both commits were created with `lisa commit-ticket` and exact include paths.

No ordinary-index staging or commit operation was used.

## Files modified

### `crates/lisa-core/src/completion.rs`

Added `CompletionDeadline`, an opaque Unix epoch millisecond value.

It supports durable reconstruction, raw access, and inclusive expiration.

Extended `CompletionState::CommandInFlight` with a deadline alongside its
mandatory CorrelationId.

Extended `CompletionEvent::CommandLaunched` with that deadline.

The reducer now installs correlation and deadline together.

Changed the pure reconciliation API to accept explicit current time.

Before deadline, in-flight reconciliation returns
`ReplayCommandInFlight`.

At or after deadline it returns
`CommandInFlightDeadlineExceeded`.

Both decisions retain exact correlation and original deadline.

Eligible, Requested, retryable/action-required Rejected, Confirmed, and
correlation mismatch behavior otherwise remain unchanged.

### `crates/lisa-core/tests/completion_state_machine.rs`

Updated the property harness with deterministic current time and deadline.

Generated event sequences still preserve one-live-effect and one-authoritative
completion invariants.

Replay-before-deadline is treated as observation of that same live effect, not
a fresh generation.

Deadline exceeded remains unreachable in this ordering-focused property because
its fixed current time is deliberately earlier.

### `crates/lisa-core/tests/recorded_livelock_regression.rs`

Updated the historical artifact-before-phase replay with deterministic deadline
values.

The prior regression still proves one request, one confirmation, no stale
finish-up prompt, and no re-request.

### `crates/lisa-plugin/src/completion_journal.rs`

Extended durable CommandInFlight transitions with `CompletionDeadline`.

New JSONL records write `reconciliation_deadline_unix_ms`.

The field is optional on read for compatibility with schema-1 journals written
before this ticket.

Missing legacy values become deadline zero.

That makes a formerly unbounded legacy in-flight state expire on its next real
reconciliation instead of silently receiving a new window.

Kept schema version 1 because the field is additive and backward-readable.

Action-required Rejected aggregates now continue masking uncertain Done bytes.

Retryable Rejected remains unmasked and can begin another request.

Added journal tests for exact deadline round-trip, legacy recovery, and both
masking cases.

### `crates/lisa-plugin/src/lib.rs`

Added a named 60-second completion reconciliation window.

Production converts `SystemTime::now()` into the core deadline type.

Tests can supply explicit time and never sleep.

Initial execution persists one deadline with CommandInFlight before the host
command is launched.

Live pending state retains the same deadline and distinguishes initial command
from reconciliation replay.

Before deadline, reconstructed in-flight reconciliation verifies the current
lease, journal key, attempt, correlation, deadline, ticket path, and absence of
another live pending invocation.

It rebuilds argv from the original `CompletionGenerationId`.

It launches through the same single host-command method as initial execution.

It does not append duplicate Requested or CommandInFlight records.

Duplicate Stop and repeated Reconcile observations remain suppressed while one
replay is pending.

A failed initial command retains the prior retryable Rejected behavior.

A failed reconciliation replay retains the original uncertain CommandInFlight
and its original deadline.

It cannot grant itself a fresh window.

At deadline, the adapter revalidates exact state and atomically appends a
correlated Rejected transition with `Retryability::ActionRequired`.

Pending state is removed only after that durable append succeeds.

Late results are ignored because no pending correlation remains.

The scheduler rebuild continues masking unconfirmed Done bytes.

## Acceptance regression

`lost_result_reload_duplicate_stop_replay_converges_on_single_prior_commit`
uses a real temporary Git repository and the real CLI library transaction.

The test drives production adapter logic through Requested and CommandInFlight.

It executes `complete_ticket` once and deliberately withholds the successful
result from the plugin.

That produces one completion commit and Done frontmatter while the journal
remains in-flight.

It constructs a fresh State and restores the exact journal aggregate.

It presents a duplicate Stop observation before reconciliation.

It then reconciles before deadline and installs one replay pending entry using
the original key.

More Stop and Reconcile observations do not add another replay.

The journal remains exactly Requested plus CommandInFlight at replay launch.

The test executes `complete_ticket` again with the same key.

The CLI returns the first commit ID with an empty committed-path list.

Git remains at base plus exactly one completion commit.

Delivering that result appends exactly one Confirmed record.

The adapter emits exactly one authoritative Done provenance row.

It releases the thread and slot once.

This distinguishes real convergence from an in-memory duplicate suppression
test.

## Deadline regression

`reconciliation_deadline_ends_action_required_without_infinite_replay` starts at
a fixed wall-clock value.

It asserts the stored deadline is exactly start plus the named 60-second bound.

It writes Done bytes while the command remains unresolved.

Reconciliation at the exact deadline appends action-required Rejected.

The live pending entry is removed.

The DAG still observes Review because uncertain Done remains masked.

Later Reconcile and duplicate Stop observations do not change launch count or
journal line count.

This proves the inclusive terminal boundary and no infinite retry.

## Verification

Formatting:

```text
cargo fmt --all -- --check
passed
```

Core:

```text
cargo test -p lisa-core
196 unit tests passed
completion state-machine property passed
recorded livelock regression passed
```

CLI:

```text
cargo test -p lisa-cli
14 transaction library tests passed
267 binary tests passed
all enabled integration tests passed
```

Plugin:

```text
cargo test -p lisa-plugin
364 passed; 0 failed
```

Workspace:

```text
cargo test --workspace
passed
```

Project gate:

```text
just check
WASM check passed
workspace tests passed
```

The environment-dependent real Zellij boundary remains intentionally ignored by
its existing test annotation.

No live provider or token-consuming test was required by this story's honest
boundary.

## Acceptance assessment

The lost-result/reload/duplicate-stop replay uses one durable key.

Met.

Replay discovers and returns the single prior completion commit.

Met with a real Git transaction and commit-count assertion.

Only one authoritative Done is published.

Met with exact Confirmed journal and provenance assertions.

CommandInFlight carries correlation and bounded deadline.

Met in core state, journal bytes, restart reconstruction, and plugin pending
state.

An unconfirmable command ends named retryable/action-required rather than
retrying forever.

Met with durable `Rejected { retryability: ActionRequired }` at the inclusive
deadline and repeated-observation suppression.

## Failure behavior reviewed

Journal append still precedes every in-memory authority transition.

A failed deadline append leaves in-flight fencing intact.

A replay command-build failure installs no pending state and cannot corrupt the
journal.

Stale lease identity prevents replay.

Mismatched correlation prevents transition.

Successful command output still requires a valid commit ID and durable Done
verification before Confirmed.

Journal health failure remains fail-closed.

Action-required timeout cannot release dependencies from uncertain Done bytes.

## Open concerns and limitations

The 60-second bound is currently a named plugin constant rather than a user
configuration field.

That is appropriate for this ticket's bounded policy but can be made
configurable in a later operational ticket if field evidence requires it.

Legacy in-flight records have no historical launch time.

They intentionally map to expired deadline zero and require action rather than
receive an invented retry window.

The journal remains whole-history atomic replacement and assumes one plugin
writer, inherited from T-042-02-02.

The regression is native and simulated-reload by story design; a live hostile
ordering run belongs to Story D.

No correctness gap, critical issue, or human-blocking TODO remains for this
ticket.

## Repository handoff

The ordinary index is empty.

All five ticket-owned source/test paths were committed through exact Lisa
transactions.

Concurrent T-042-03-02 changes in shared `crates/lisa-plugin/src/lib.rs` and
`ui.rs` occurred after this ticket's plugin commit and passed the combined
workspace gates.

They are not part of this ticket's commits.

Lisa-owned provenance, ticket frontmatter, admitted canonical work artifacts,
and the unrelated untracked plugin docs tree were not committed by this ticket.

This Review is ready for Lisa admission and the final completion transaction.

The agent must remain on T-042-02-03 until Lisa confirms that transaction.
