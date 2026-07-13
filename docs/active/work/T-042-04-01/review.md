# Review: hostile-order real-adapter regression

## Review outcome

The ticket is ready to complete.

The deterministic regression drives the real `lisa-plugin` adapter through
the recorded hostile ordering in a two-level nested-monorepo fixture.

The passing path converges on one durable completion generation, one Git
completion commit, one confirmation, one authoritative Done record, seat
release, and dependent scheduling.

The blocked path produces none of those completion side effects.

Both paths prove that an already-present Review suppresses the false finish-up
prompt while completion is pending, actionable, or refused by disposition.

No production behavior changed.

No critical issue remains open.

## Source commit

The meaningful source unit was committed through Lisa's isolated transaction.

Commit:

`079436699659e6897ad761b5cb608b8490a7f0e2`

Message:

`test(plugin): replay hostile completion order`

The PATH-installed Lisa binary was older and lacked `commit-ticket`.

The repository-built `target/debug/lisa commit-ticket` command was used.

The command received exact includes for only:

- `crates/lisa-plugin/src/lib.rs`;
- `crates/lisa-plugin/src/tests/hostile_order_regression.rs`.

`git diff-tree` confirms those are the only commit paths.

No ordinary Git staging or commit command was used for ticket source.

## Files changed

### `crates/lisa-plugin/src/lib.rs`

Added one declaration inside the existing `#[cfg(test)]` module:

`mod hostile_order_regression;`

No production line changed.

No public or private production interface changed.

### `crates/lisa-plugin/src/tests/hostile_order_regression.rs`

Added a 493-line focused native test module.

The module contains one nested repository helper, one shared scenario fixture,
small command-decoding helpers, and two acceptance tests.

The fixture uses existing private plugin adapter methods through the parent
native test module.

It calls the exported production CLI transaction directly.

## Nested topology

Every scenario initializes a fresh Git repository.

The Lisa project is located at `games/midsummer`.

The primary ticket is located at:

`games/midsummer/docs/active/tickets/T-ARCADE-PRIMARY.md`.

The dependent ticket is in the same nested ticket directory.

The primary begins in Implement.

The dependent begins Ready with an explicit dependency on the primary.

Both are committed as the fixture baseline.

The test uses actual temporary absolute paths for native file access.

The real command builder normalizes those paths to Git-root-relative argv.

## Real adapter coverage

The regression calls `State::check_artifact_advances`.

It calls `State::handle_stopped_signal` for the transition Stop.

It calls `State::check_review_timeouts` for timeout policy.

It drives `d` then Enter through `State::handle_key`.

It restores the real completion journal into a fresh `State`.

It calls typed `CompletionInput::Reconcile` through the production dispatcher.

It calls `State::handle_completion_result` for durable confirmation.

It observes the production release and scheduler paths.

This is adapter coverage rather than another reducer-only regression.

## Review-before-transition ordering

The fixture writes private `review.md` and disposition before advancement.

The primary thread still reports Implement at that point.

The real artifact fixpoint admits Review evidence and advances to Review.

On the same fixpoint, Review eligibility reaches typed completion dispatch.

The on-disk ticket remains Review before the isolated transaction.

This reproduces Review existing before and through Implement-to-Review.

## Stop during slot transition

The primary seat begins in WaitingForStop.

After completion intent exists, the test delivers Stop through the real handler.

The handler moves the seat to WaitingForClear.

It does not create a second completion effect or generation.

The blocked scenario drives the same transition and remains effect-free.

## Nested command contract

The passing test builds argv from the live pending completion key.

It asserts `--path` is the temporary Git root.

It asserts `--ticket-file` is:

`games/midsummer/docs/active/tickets/T-ARCADE-PRIMARY.md`.

It asserts `--work-dir` is:

`games/midsummer/docs/active/work/T-ARCADE-PRIMARY`.

It also checks the `lisa_completion` command context.

The real `complete_ticket` implementation consumes that request.

## Passing transaction behavior

The first transaction changes durable ticket frontmatter to Done.

It creates exactly one Git commit above the baseline.

The test checks that commit's parent is the baseline HEAD.

The successful command result is intentionally delayed across plugin reload.

Before confirmation, the journal still contains only Requested and
CommandInFlight.

No authoritative provenance exists at that point.

## Reload, delayed result, and duplicates

A fresh plugin state restores the exact durable aggregate.

Unconfirmed Done bytes are masked back to Review scheduler authority.

A duplicate Stop before replay creates no effect.

Reconciliation before the deadline replays the original completion key.

The replay's effect identity equals the initial effect identity exactly.

No new Requested or CommandInFlight record is appended.

Further duplicate Stop and Reconcile observations create nothing further.

The same CLI request returns the original commit with no committed paths.

The repository remains baseline plus exactly one completion commit.

The correlated successful result is then delivered twice.

Only the first delivery appends Confirmed and releases scheduler state.

The late duplicate result is inert because no pending correlation remains.

## Exactly-one passing outcome

The final journal contains exactly one Confirmed record.

The provenance ledger contains exactly one execution record.

That record is Done and authoritative.

The primary thread is removed.

The current primary lease is revoked.

No slot retains the primary ticket.

The dependent receives a thread and seat reservation immediately.

The transaction commit count remains exactly one.

The durable completion generation remains exactly one across replay.

## Attempted `[d]one`

The passing sequence attempts `[d]one` while completion is pending.

The modal reports the named AlreadyPending rejection.

No second effect is created.

The blocked sequence attempts `[d]one` after Review admission.

The operator path cannot bypass the Block disposition.

The actionable block reason remains in structured rejection activity.

## Blocked outcome

The blocked Review advances Implement to Review and no further.

Artifact, Stop, Reconcile, timeout, and operator observations create no
completion intent.

The completion journal is not created.

The provenance ledger is not created.

Git HEAD and commit count remain at the fixture baseline.

The ticket remains Review and is never Done.

The primary thread, lease, and seat remain assigned.

The dependent remains unscheduled.

This proves the required “none” outcome through the real adapter.

## Finish-up suppression

Both scenarios age Review clocks beyond timeout and wind-down thresholds.

The passing scenario checks suppression with a live pending completion.

The blocked scenario checks suppression with admitted Review evidence and no
pending completion.

Neither scenario records `FinishUpPromptSent`.

Neither scenario inserts the ticket into `finish_up_sent`.

The regression therefore rejects the field-observed false follow-up shape.

## Verification results

Focused regression:

`cargo test -p lisa-plugin --lib hostile_order_regression --no-fail-fast`

Passed: 2; failed: 0.

Full plugin library:

`cargo test -p lisa-plugin --lib --no-fail-fast`

Passed: 373; failed: 0.

Full workspace:

`cargo test --workspace --no-fail-fast`

Passed across CLI library/binary/integration, core, plugin, and doctests.

Project check:

`just check`

The WASM target check and workspace tests passed.

Formatting and hygiene:

- `cargo fmt --all -- --check` passed;
- `git diff --check` passed;
- the ordinary Git index is empty;
- ticket-owned source paths are clean.

## Coverage assessment

The acceptance sequence is covered without sleeps or a live provider.

Explicit time and direct adapter observations keep the test deterministic.

The pure reducer remains covered by its existing property and recorded-order
tests; this ticket adds the missing composition layer.

The real CLI transaction supplies actual Git commit and idempotency evidence.

The plugin's Zellij host function remains a native no-op, as in the rest of
the deterministic adapter suite.

The later `T-042-04-03` ticket owns a freshly built live Codex-seat field run.

## Open concerns and limitations

No blocking concern exists.

The regression directly invokes private adapter methods rather than starting a
real Zellij server.

That is deliberate: it exercises the real state adapter deterministically,
while live runtime evidence is a separate story ticket.

Replay invokes the same host effect identity a second time after reload, as
required for lost-result convergence.

Exactly-one is enforced at the durable generation, intent, Git commit,
confirmation, provenance, release, and scheduling boundaries.

## Repository preservation

Lisa-managed ticket and provenance modifications were not included in source.

The unrelated untracked `crates/lisa-plugin/docs/` tree was preserved.

The shared `docs/active/work/T-042-04-01/` path was not authored by this
attempt and was not included in the source commit.

All phase artifacts were authored in the private attempt work directory.

## Final disposition

Pass.

The acceptance criterion is satisfied, verification is green, ticket-owned
source is committed and clean, and no critical issue requires human action.

This attempt remains on `T-042-04-01` for Lisa to admit Review, publish the
final completion transaction, release the seat, and schedule dependents.

