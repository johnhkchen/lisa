# Progress: hostile-order real-adapter regression

## Status

Implementation is complete.

The focused hostile-order regression passes.

The full plugin and workspace suites pass.

Formatting, diff, and WASM checks pass.

The ticket-owned source unit is ready for its isolated Lisa commit.

## Completed phase work

Read `CLAUDE.md`, `AGENTS.md`, the ticket, assignment, and RDSPI workflow.

Mapped the core reducer, plugin adapter, journal, transaction, timeout,
operator, provenance, release, and scheduler boundaries.

Read the completed dependency reviews for `T-042-01-07`, `T-042-02-03`,
and `T-042-03-03`.

Wrote Research, Design, Structure, and Plan artifacts in the private attempt
work directory.

Did not edit ticket phase or status.

## Source changes

Modified `crates/lisa-plugin/src/lib.rs`.

The only change is a native test-module declaration:
`mod hostile_order_regression;`.

Created
`crates/lisa-plugin/src/tests/hostile_order_regression.rs`.

The new file contains a two-level nested-monorepo fixture and two acceptance
regressions.

No production method, type, constant, or branch changed.

No core, CLI, manifest, lockfile, or UI file changed.

## Fixture implementation

Added `NestedRepo`, which initializes a disposable Git repository.

It configures deterministic test author identity.

It places the Lisa project at `games/midsummer`.

It provides checked write, Git command, Git stdout, and commit-count helpers.

Added `Scenario`, which creates two real ticket files.

The primary begins in Implement.

The dependent begins Ready and depends on the primary.

The fixture commits those inputs as a baseline.

It scans tickets through `lisa_core::ticket::scan_tickets`.

It constructs the production `Dag` and plugin `State`.

It configures real journal, provenance, attempts, signals, and usage paths.

It installs a Running Codex thread and exact current attempt lease.

It installs a primary seat in WaitingForStop.

It installs a spare compatible seat for observable dependent scheduling.

It enables the same scheduler permission/discovery gates used in production.

It writes private `review.md` and structured disposition before advancement.

## Passing sequence implemented

`passing_review_hostile_order_converges_once_and_schedules_dependent` starts
with private Review evidence already present during Implement.

It calls the real `check_artifact_advances` fixpoint.

The thread and durable ticket advance to Review.

The real typed Artifact path produces one initial completion effect.

The journal contains exactly Requested and CommandInFlight.

The ticket remains non-Done before the transaction.

The test calls `handle_stopped_signal` while the seat is WaitingForStop.

The seat moves to WaitingForClear and no new completion effect appears.

It ages the Review clocks and calls `check_review_timeouts`.

No FinishUpPromptSent event or marker appears.

It submits `d` then Enter through `handle_key`.

The modal shows named AlreadyPending rejection.

The initial effect count remains one.

## Real nested command and transaction

The test obtains the live completion key from adapter pending state.

It calls the production `State::build_completion_command`.

It asserts `--path` equals the temporary Git root.

It asserts `--ticket-file` equals
`games/midsummer/docs/active/tickets/T-ARCADE-PRIMARY.md`.

It asserts `--work-dir` equals
`games/midsummer/docs/active/work/T-ARCADE-PRIMARY`.

It calls the exported production `complete_ticket` transaction.

The first transaction advances Git by exactly one commit.

The completion commit has the fixture baseline as its parent.

Durable ticket bytes become Done.

The successful result is deliberately withheld from the first plugin state.

No provenance is emitted before adapter confirmation.

## Reload and hostile duplicates

The test constructs a fresh `State` from durable paths.

It restores the real completion journal before rebuilding the DAG.

It reinstalls the exact attempt lease, thread, primary seat, and spare seat.

Raw Done bytes remain masked as Review while the journal is unresolved.

A duplicate Stop observation before replay emits no new effect.

Reconcile before the stored deadline replays the original generation.

The replay effect has exactly the same attempt and completion identity as the
initial effect.

The pending replay has the exact original completion key.

Further duplicate Stop and Reconcile observations emit nothing further.

The journal remains at two intent/in-flight records.

The same real transaction request is executed again.

It returns the original commit and an empty committed-path list.

The repository remains baseline plus one commit.

The correlated result is delivered twice.

Only the first delivery appends Confirmed.

The second delayed duplicate is inert because no pending result remains.

## Passing outcome assertions

The final journal has exactly three records.

Exactly one record is Confirmed.

The provenance ledger has exactly one execution row.

That row is Done and authoritative.

The primary thread is removed.

The primary current lease is revoked.

No seat retains the primary ticket.

The dependent thread is created.

An eligible seat is reserved for the dependent.

No false finish-up prompt appears anywhere in the sequence.

## Blocked sequence implemented

`blocked_review_hostile_order_has_no_completion_side_effects` uses the same
nested fixture with a valid Block disposition and actionable reason.

Private Review exists before Implement advances.

Artifact advancement reaches Review but produces no pending completion.

Stop during WaitingForStop produces no completion effect.

Explicit Reconcile produces no completion effect.

Expired Review timeout produces no finish-up prompt because Review exists.

Attempted `d` then Enter cannot bypass the Block disposition.

No completion journal is created.

No provenance ledger is created.

No Git completion commit is created.

The ticket remains Review and non-Done.

The primary thread, lease, and seat remain assigned.

The dependent remains unscheduled.

The actionable block reason remains visible in structured rejection activity.

## Deviations from Plan

The native fixture uses its actual temporary absolute paths for configured
ticket and work directories rather than synthetic `/host` filesystem paths.

This is required so native artifact admission and the real CLI transaction can
read the same temporary files.

The production path normalizer is still exercised by the real command builder,
and the asserted argv is the required Git-root plus
`games/midsummer/docs/...` contract.

No scope or production behavior changed because of this fixture adjustment.

No production defect was found.

## Verification

Focused module:

`cargo test -p lisa-plugin --lib hostile_order_regression --no-fail-fast`

Result: 2 passed; 0 failed.

Full plugin library:

`cargo test -p lisa-plugin --lib --no-fail-fast`

Result: 373 passed; 0 failed.

Full workspace:

`cargo test --workspace --no-fail-fast`

Result: passed across CLI, core, plugin, integration, and doctest targets.

Formatting and diff:

`cargo fmt --all -- --check`

`git diff --check`

Both passed.

Project gate:

`just check`

The WASM check and full workspace tests passed.

## Repository hygiene

The ordinary Git index is empty.

Lisa-managed `.lisa/provenance.jsonl` and active ticket changes were preserved.

The unrelated untracked `crates/lisa-plugin/docs/` tree was preserved.

Lisa's automatically published `docs/active/work/T-042-04-01/` path was not
written by this attempt and was not included in ticket source ownership.

Only the two planned plugin test paths are ticket-owned source.

## Isolated source commit

The PATH-installed `lisa` rejected `commit-ticket` because it is an older CLI.

The repository-built `target/debug/lisa` exposes the required command and was
used for the same isolated transaction.

Exact command ownership was limited to:

- `crates/lisa-plugin/src/lib.rs`;
- `crates/lisa-plugin/src/tests/hostile_order_regression.rs`.

The resulting commit is:

`079436699659e6897ad761b5cb608b8490a7f0e2`

Commit message:

`test(plugin): replay hostile completion order`

## Remaining work

Write Review and the required disposition JSON.

Remain on this ticket for Lisa's completion gate.

The source commit was inspected with `git show` and `git diff-tree`.

It contains exactly the two planned paths and 494 test-only inserted lines.

Both ticket-owned source paths are clean after the transaction.

The ordinary index remains empty.
