# Structure: hostile-order real-adapter regression

## File changes

Modify `crates/lisa-plugin/src/lib.rs` only inside its native test module.

Add `mod hostile_order_regression;` beside focused test modules.

Create `crates/lisa-plugin/src/tests/hostile_order_regression.rs`.

Do not modify production modules.

Do not modify `lisa-core` or `lisa-cli`.

Do not modify manifests or `Cargo.lock`.

Do not delete any file.

## Test module boundary

The new module imports its parent with `use super::*`.

It remains native-test-only through the parent `#[cfg(test)]` boundary.

It uses private adapter methods without widening production visibility.

It imports `complete_ticket` and `CompleteTicketRequest` from `lisa-cli`.

The existing dev-dependency supports that import.

## Constants

Define stable primary and dependent ticket IDs.

Define stable pane IDs for the completing and spare seats.

Use descriptive IDs tied to the Arcade regression rather than production IDs
that could collide with fixture assumptions.

## Nested repository helper

Own a `tempfile::TempDir`.

Expose the Git root.

Expose the nested project root at `games/midsummer`.

Provide a path-aware write helper.

Provide a checked Git command helper.

Provide a checked Git stdout helper.

Keep subprocess usage inside the fixture boundary.

## Completion argv helper

Provide a small option lookup over adjacent argv entries.

Decode only the exact `complete-ticket` arguments produced by the adapter.

Build `CompleteTicketRequest` using the live completion key.

Assert the request root equals the fixture Git root.

Assert ticket and work paths begin with `games/midsummer/docs/active`.

Do not duplicate general Clap parsing.

## Scenario fixture

Hold the repository helper and plugin `State`.

Hold the installed current `AttemptLease`.

Hold baseline HEAD and commit count.

Initialize the primary ticket in Implement and dependent in Ready.

Initialize canonical directories and journal/ledger locations.

Install the primary thread at Implement.

Install the completing slot in WaitingForStop.

Install one free compatible slot for the dependent.

Enable scheduler prerequisites.

Set Review timeout and wind-down values for deterministic expiry.

## Evidence writers

Write private `review.md` in `State::attempt_work_dir`.

Write private `review-disposition.json` beside it.

Parameterize only the disposition bytes.

Use the same Review content for Pass and Block.

Ensure evidence exists before calling phase advancement.

## Sequence methods

Provide a method to advance artifact state and inspect the initial adapter
outcome.

Provide a method to drive Stop in WaitingForStop.

Provide a method to age Review clocks and run timeout checking.

Provide a method to drive `d` then Enter.

Provide a method to create a restarted state from durable paths and authority.

Keep assertions in tests when they express acceptance semantics.

Keep fixture mechanics in helpers.

## Passing test

Name the test for hostile ordering and exactly-one passing completion.

Assert Review existed before phase advancement.

Assert Implement advances to Review and one initial effect is emitted.

Assert Stop-transition handling does not duplicate completion.

Assert timeout does not emit FinishUpPromptSent.

Assert `[d]one` is rejected as AlreadyPending without another effect.

Assert command argv selects Git root and nested paths.

Execute the real transaction once.

Restore a fresh state before delivering the result.

Assert raw Done is masked while journal state is unresolved.

Replay with the same key and suppress duplicate observations.

Execute idempotent transaction replay.

Confirm through `handle_completion_result`.

Deliver the same result again to prove late duplication is inert.

Assert one commit, one confirmation, one authoritative provenance row,
one release, and dependent scheduling.

## Blocked test

Name the test for hostile ordering and zero blocked completion effects.

Use a valid Block disposition with an actionable reason.

Assert artifact advancement stops at Review.

Assert Artifact, Stop, Reconcile, timeout, and operator observations do not
create completion intent or Done.

Assert no journal or provenance file is created with completion records.

Assert the repository remains at baseline HEAD.

Assert the primary retains its thread, lease, and slot.

Assert the dependent remains ready but unscheduled.

Assert the block reason is present in correlated activity.

Assert no false finish-up marker or event appears.

## Shared assertions

Count exact `EffectCommand::LaunchCompletion` values.

Count journal states by parsing or stable JSON state labels.

Parse mixed provenance with the existing parent helper.

Inspect ticket frontmatter through the real scanner or file bytes.

Inspect slot release through ticket and attempt fields.

Inspect dependent scheduling through its thread and slot reservation.

Inspect unique completion identity across initial state and restart.

## Production boundaries preserved

`State::dispatch_completion` remains the sole adapter request gateway.

`State::execute_completion_effect` remains the sole new-effect executor.

`complete_ticket` remains the real isolated transaction.

`handle_completion_result` remains the only confirmation/release crossing.

`schedule_ready_tickets` remains the dependent scheduling boundary.

## Commit ownership

The module declaration and new module form one meaningful source unit.

Commit both with one `lisa commit-ticket` transaction.

Use exact includes for only those two repository-relative paths.

Attempt artifacts are excluded from the source transaction.

Lisa-managed ticket/provenance changes and plugin docs remain untouched.

