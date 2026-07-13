# Design: hostile-order real-adapter regression

## Objective

Add deterministic evidence that the real plugin adapter converges the Arcade
hostile sequence exactly once for Pass and never for Block.

Keep production completion behavior unchanged.

Use one nested-repository fixture shared by explicit passing and blocked tests.

## Option 1: extend the pure core regression

The existing `recorded_livelock_regression` already models hostile ordering.

It is fast and has explicit deterministic time.

It can count reducer effects and confirmations precisely.

It cannot prove plugin path construction.

It cannot prove journal reconstruction in `State`.

It cannot prove slot release, provenance, or dependent scheduling.

It cannot drive the `[d]one` keyboard path.

This option does not satisfy the real-adapter acceptance boundary.

## Option 2: add isolated assertions to existing plugin tests

Separate current tests already cover nested paths, timeout suppression,
replay, operator input, disposition, and scheduling.

Adding one assertion to each would minimize new fixture code.

The resulting evidence would not replay a single hostile order.

It would permit incompatible assumptions between fixtures.

It would not show the same completion key crossing effect, transaction,
reload, duplicate observations, and confirmation.

This option is useful background coverage but not the requested regression.

## Option 3: use an external integration test crate

An integration test could initialize Git and invoke public APIs.

It would have a clean black-box file boundary.

Most plugin scheduler methods and state fields are intentionally private.

Exposing them would broaden production API only for testing.

Driving Zellij host events externally would add runtime nondeterminism.

The ticket asks for the real adapter, not necessarily a real Zellij server.

This option creates unnecessary API and environment cost.

## Option 4: focused native submodule under plugin tests

A child module of the existing `#[cfg(test)] mod tests` can use private state.

It can drive the exact plugin methods used by production polling.

It can call the exported real CLI transaction deterministically.

It can inspect launch effects, journal bytes, pending identity, slots,
threads, activity, modal state, and provenance.

It can share one nested fixture while keeping `lib.rs` from growing further.

This matches the established operator matrix organization.

This is the selected approach.

## Fixture shape

Create a temporary Git repository.

Place the Lisa project exactly at `games/midsummer`.

Create a primary ticket initially in Implement.

Create a dependent Ready ticket with `depends_on` on the primary.

Commit both ticket files as the baseline tree.

Configure the plugin with host-view ticket/work paths under `/host/docs`.

Set `project_root` to the nested Lisa project.

Set `git_root` to the temporary repository root.

Set a real journal and provenance path in the nested `.lisa` directory.

Install a Running Codex thread, current attempt lease, and provider slot.

Add a second idle slot so dependent scheduling is observable without relying
on completion cooldown timing.

Enable permissions and slot discovery for the scheduler.

## Passing sequence

Write private `review.md` and a Pass disposition before phase advancement.

Put the primary slot in WaitingForStop to represent the hostile transition.

Call `check_artifact_advances`.

This crosses Implement to Review and dispatches the sole completion effect.

Assert the on-disk ticket is Review, not Done.

Assert exactly one launch effect and one journal intent/in-flight pair.

Derive command argv through `build_completion_command` using the live key.

Assert Git-root and nested repository-relative options exactly.

Call `handle_stopped_signal` while the slot is WaitingForStop.

Assert it advances the transition but creates no second effect.

Expire Review activity clocks and call `check_review_timeouts`.

Assert no finish-up marker or event appears while completion is pending.

Attempt `[d]one` through the real key handler.

Assert AlreadyPending remains visible and creates no second effect.

Execute the real CLI transaction and deliberately delay its result.

Assert exactly one Git commit and durable Done bytes.

Create a fresh plugin state and restore the journal.

Reinstall the exact current lease, thread, and slot records.

Rebuild the DAG so unconfirmed Done remains masked as Review.

Deliver duplicate Stop before replay and assert no effect.

Reconcile before deadline to replay the original generation once.

Deliver further duplicate Stop/Reconcile observations and assert suppression.

Execute the same CLI request again and assert it discovers the prior commit.

Deliver the replay result and then deliver it again.

Assert exactly one Confirmed journal record and one authoritative Done row.

Assert the original seat is released and the dependent is scheduled.

Assert no finish-up prompt was ever emitted.

## Blocked sequence

Use the same nested fixture and ordering with a Block disposition.

Write private Review before Implement-to-Review.

Run artifact advancement.

The disposition gate must refuse completion.

Drive Stop during the transition and a later Reconcile observation.

Attempt `[d]one` through the key handler.

Expire timeout clocks and run Review timeout policy.

Assert the admitted Review suppresses the generic prompt.

Assert zero launch effects, zero journal transitions, zero completion commits,
zero authoritative Done records, no release, and no dependent scheduling.

Assert the blocking reason remains visible.

## Transaction counting

Record baseline HEAD and commit count after committing fixture inputs.

For Pass, require HEAD count to advance by exactly one.

Require the returned commit to have the baseline as its parent.

Require idempotent replay to return the same commit and empty committed paths.

For Block, require HEAD and commit count to remain unchanged.

## Effect and authoritative outcome counting

Count `launched_completion_effects` as adapter launch observations.

Across reload, initial and replay launch are host invocations of one durable
generation, not two domain completion effects.

Therefore assert one unique completion key/correlation and no second intent.

Count journal `confirmed` lines for authoritative aggregate confirmation.

Count execution provenance records whose outcome is Done and authoritative.

Both counts must be exactly one for Pass and zero for Block.

## Rejected alternatives

Do not add sleeps; use explicit timestamps and direct timeout clock setup.

Do not launch a live Codex process; the story assigns that to a later ticket.

Do not mock the CLI commit result without running `complete_ticket`.

Do not modify the reducer or adapter to make the fixture easier.

Do not add a production test-only public API.

## Expected source unit

Register one `hostile_order_regression` child module in `lib.rs` tests.

Create `crates/lisa-plugin/src/tests/hostile_order_regression.rs`.

Treat both paths as one atomic test-owned source unit.

