# Progress: restart reconstruction and lost-result fixtures

## Current status

Implementation is complete.

The ticket-owned source unit is committed.

All focused, plugin, workspace, and WASM checks pass.

No production contract changed.

No blocking defect was discovered.

## Plan execution

The implementation followed the recorded plan.

The predecessor nested-repository harness was extended in place.

No new module registration was necessary.

No static journal data file was necessary.

No production visibility or public API was expanded.

No deviation affecting scope or acceptance occurred.

The focused `fixture` filter also selected four existing Codex ack fixture
tests, as anticipated by the plan; all six selected tests passed.

## Source file

Modified:

`crates/lisa-plugin/src/tests/hostile_order_regression.rs`.

The file remains a private child of the plugin's native test module.

It continues using the real adapter methods through `super::*`.

It continues calling the exported real CLI completion transaction.

No other source file changed for this ticket.

## Shared fixture implemented

Added private `LostResultFixture`.

The fixture owns the existing nested `Scenario`.

It captures the original `PendingCompletion`.

It captures the original typed `EffectCommand`.

It captures the first transaction's real Git commit ID.

Its construction creates a passing Review in `games/midsummer`.

The project remains two levels below its temporary Git root.

The primary ticket advances through the real artifact adapter.

The adapter publishes one Requested journal transition.

The adapter publishes one CommandInFlight journal transition.

The fixture checks the aggregate uses the pending generation.

It checks the aggregate correlation equals pending correlation.

It checks the aggregate deadline equals pending deadline.

It checks exactly one typed effect was launched.

## Lost-result boundary

Fixture construction builds the CLI request from adapter-generated argv.

The existing command helper checks `--path` is the Git root.

It checks `--ticket-file` is the nested repository-relative primary path.

It checks `--work-dir` is the nested repository-relative work path.

It calls `complete_ticket` once with the original generation.

The CLI creates a real completion commit.

The fixture checks that commit has the fixture baseline as its parent.

It checks the committed ticket contains `phase: done`.

It deliberately does not deliver the successful command result.

The journal therefore stays at two records.

The aggregate therefore stays CommandInFlight.

The provenance ledger therefore does not yet exist.

This is the shared durable lost-result prefix.

## Restart reconstruction helper

Added `restart_in_flight`.

It constructs a fresh plugin `State` from durable scenario paths.

It restores the production completion journal.

It rebuilds the production DAG with completion masking.

It reinstalls deterministic current attempt authority from the fixture.

It asserts journal restoration is healthy.

It asserts no process-local pending completion survived restart.

It asserts the reconstructed aggregate has the original generation.

It asserts reconstructed state is exact CommandInFlight.

It asserts correlation is the original correlation.

It asserts deadline is the original absolute deadline.

It asserts `reconciliation_state` reports the same typed state.

It asserts durable Done bytes remain masked to the prior phase and status.

## Bounded replay helper

Added `replay_time`.

It derives explicit adapter time from deadline minus one millisecond.

It does not use a sleep.

It does not depend on test execution speed.

Added `start_replay`.

It dispatches real typed Reconcile input.

It uses the current fixture lease.

It proves exactly one replay effect is launched.

It proves that effect equals the initial effect.

It proves the replay pending entry retains the original generation.

It proves the replay retains the original correlation.

It proves the replay retains the original absolute deadline.

It proves the pending entry is marked reconciliation replay.

It proves replay appends no new Requested or CommandInFlight journal record.

## Exactly-once convergence helper

Added `converge`.

It executes the real CLI transaction a second time with the original key.

The repeat returns the first completion commit ID.

The repeat reports an empty committed-path set.

The repository remains baseline plus one commit.

The replay result is delivered through the real adapter result handler.

The durable aggregate ends in named state Confirmed.

The aggregate records the first commit ID.

The journal ends with exactly three records.

It has one Requested record.

It has one CommandInFlight record.

It has one Confirmed record.

The provenance ledger ends with exactly one record.

That record is an Execution record for the primary ticket.

Its outcome is Done.

It is authoritative.

The repository commit count remains baseline plus one after confirmation.

## Restart fixture test

Added:

`plugin_restart_reconstruction_fixture_converges_on_single_prior_commit`.

The test starts at the real lost-result boundary.

It creates a fresh plugin adapter state.

It proves exact CommandInFlight reconstruction.

It starts one replay inside the retained deadline.

It converges on the first completion commit.

It creates another fresh plugin state after confirmation.

That second restart reconstructs Confirmed.

It reconstructs the same first commit ID.

Its reconciliation state is Confirmed.

This proves both in-flight and successful terminal durability across restart.

## Duplicate-Stop fixture test

Added:

`lost_result_duplicate_stop_fixture_converges_on_single_prior_commit`.

The test starts at the same real lost-result boundary.

It creates a fresh plugin adapter state.

It sends two Stop observations before replay.

No pending invocation is created.

No effect is launched.

Journal bytes remain unchanged.

It then starts one bounded replay.

It sends two more Stop observations while replay is pending.

It sends a repeated Reconcile observation while replay is pending.

No second effect is launched.

Journal bytes remain unchanged at two records.

The real CLI repeat returns the first commit.

Confirmation creates one authoritative Done.

The test then delivers the same success result again.

The duplicate result changes neither journal nor provenance bytes.

The repository remains at one completion commit.

## Focused verification

Command:

`cargo test -p lisa-plugin --lib fixture --no-fail-fast`.

Result:

6 passed; 0 failed.

The six include both new tests and four existing Codex fixture tests.

Command:

`cargo test -p lisa-plugin --lib hostile_order_regression --no-fail-fast`.

Result:

4 passed; 0 failed.

Both new focused fixtures passed.

Both predecessor hostile-order cases passed.

## Plugin verification

Command:

`cargo test -p lisa-plugin --lib --no-fail-fast`.

Result:

375 passed; 0 failed.

This includes existing journal, deadline, adapter, and scheduler regressions.

## Source commit

The source unit was committed using the repository-built Lisa CLI.

No ordinary Git add or commit command was used.

Command contract:

`lisa commit-ticket --ticket-id T-042-04-02 --message "test(plugin): add restart replay fixtures" --include crates/lisa-plugin/src/tests/hostile_order_regression.rs`.

Commit:

`33d339a878a38d9e6fd4014e02a5cd9dcb042e67`.

`git diff-tree` reports exactly one path:

`crates/lisa-plugin/src/tests/hostile_order_regression.rs`.

The commit contains 259 inserted test lines.

The ordinary Git index remained empty.

## Workspace verification

Command:

`cargo test --workspace --no-fail-fast`.

Result:

Passed across CLI library, CLI binary, integration tests, core, plugin, and
doctests.

Command:

`just check`.

Result:

Passed.

The WASM target `cargo check` passed.

The workspace tests run by `just check` passed.

## Formatting and hygiene

`cargo fmt --all` formatted the new test code.

`cargo fmt --all -- --check` passed before commit.

`git diff --check` passed before commit.

The ticket-owned test file is clean after commit.

The ordinary index is empty.

Lisa-managed ticket, provenance, and shared artifact paths remain outside the
ticket-owned source commit.

The unrelated pre-existing untracked `crates/lisa-plugin/docs/` tree remains
preserved and untouched.

## Remaining work

Perform the final post-verification hygiene audit.

Write `review.md` in the private attempt directory.

Write the exact valid `review-disposition.json` shape.

Remain on this ticket for Lisa's completion transaction.
