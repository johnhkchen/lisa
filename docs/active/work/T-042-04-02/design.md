# Design: restart reconstruction and lost-result fixtures

## Goal

Add focused regression evidence for the two durable-reality cases named by
`T-042-04-02`.

The fixtures must use the real plugin adapter and real CLI Git transaction.

They must run in the nested repository topology established by the predecessor.

They must demonstrate identity convergence, not only effect suppression.

They must leave production behavior unchanged.

## Evidence required from each fixture

Each successful fixture needs evidence at four boundaries.

The journal boundary must reconstruct the exact CommandInFlight generation.

The adapter boundary must replay the exact correlation and retained deadline.

The Git boundary must return the existing commit and keep commit count stable.

The authority boundary must emit one authoritative Done and end Confirmed.

No one boundary is sufficient on its own.

An in-memory effect count cannot prove Git idempotency.

A stable Git count cannot prove adapter confirmation or authoritative release.

A journal state assertion cannot prove the real command path was exercised.

The chosen tests therefore compose all four.

## Option 1: keep relying on existing broad tests

The predecessor passing test already includes a lost result and reload.

An older inline test also covers lost result, duplicate Stop, and real Git.

The current suite is green and production behavior is already implemented.

This option would make no source change.

It would not add the focused fixture deliverable named by this ticket.

The predecessor sequence combines many story requirements in one test.

The older inline test uses a root-level project instead of the story topology.

Reviewers cannot independently select restart reconstruction and duplicate Stop
as named fixture cases from the predecessor module.

This option is rejected because it leaves the ticket-specific evidence implicit.

## Option 2: add static journal JSON fixture files

Two JSONL files could encode Requested and CommandInFlight records.

Tests could copy them into a temporary repository and restore a fresh State.

This would make the input bytes easy to inspect.

It would also create stable recorded inputs for parser reconstruction.

However, completion generations contain attempt identity.

CommandInFlight records contain correlations derived from that generation.

They also contain absolute deadlines.

Real completion commits contain identity trailers and generated commit IDs.

A static journal would either hard-code these values or require rewriting them.

Most importantly, starting from static CommandInFlight bytes would skip the
real adapter's initial Requested-to-CommandInFlight publication boundary.

The journal module already has byte-level reconstruction tests.

This ticket asks for real-adapter fixtures rather than more parser fixtures.

This option is rejected as the primary approach.

## Option 3: create a new standalone integration test target

A file under `crates/lisa-plugin/tests/` could model restart and replay.

That would give the cases a conventional integration-test boundary.

It could call public crate interfaces only.

The relevant adapter methods and State internals are intentionally private.

Making them public would expand production API solely for tests.

Driving only exported ZellijPlugin methods would require host runtime plumbing.

The existing native unit-test seam deliberately avoids that infrastructure.

This option is rejected because it either weakens encapsulation or introduces
an unrelated runtime harness.

## Option 4: add another independent native test module

A sibling module could duplicate the nested Git scenario and private setup.

It would retain access to adapter internals through `super::*`.

The focused cases would have clear names and isolated source organization.

The setup would repeat roughly two hundred lines from the predecessor harness.

That duplicated setup includes topology, tickets, paths, thread, slot, lease,
artifact admission, command decoding, and Git helpers.

Drift between the story tests would make future changes harder to interpret.

This option is viable but unnecessarily repeats the exact contract under test.

It is rejected in favor of extending the established harness.

## Option 5: extend the predecessor harness with focused fixtures

The existing `hostile_order_regression` module already owns the story topology.

It already derives transaction requests from real adapter argv.

It already knows how to construct a fresh State from durable paths.

A small fixture can drive the common lost-result prefix once per test:

- admit passing Review evidence through the adapter;
- capture the exact pending generation, correlation, deadline, and effect;
- run the real completion transaction once;
- intentionally withhold the result;
- retain the resulting prior commit identity.

Two tests can then apply different observation suffixes.

The restart fixture can focus on exact reconstruction and final reconstruction.

The duplicate-Stop fixture can focus on suppression before and during replay.

Both can use a common convergence assertion.

This approach adds only test code.

It preserves production encapsulation.

It retains the nested Git-root-relative command contract.

It is the chosen design.

## Chosen fixture model

Add a private `LostResultFixture` to the predecessor test module.

The fixture owns `Scenario` so its temporary repository stays alive.

It stores the original pending completion.

It stores the original typed launch effect.

It stores the first transaction's commit ID.

Construction uses a passing Review disposition.

Construction calls the real artifact advancement adapter.

It asserts Review has been reached and exactly one launch exists.

It asserts the durable aggregate is CommandInFlight with the pending values.

It runs `complete_ticket` from adapter-generated argv.

It withholds `handle_completion_result` to model the lost result.

It asserts one commit above the baseline and no provenance yet.

## Exact reconstruction assertions

A fresh state must report a healthy journal.

It must have no live pending completion before reconciliation.

Its aggregate key must equal the original generation.

Its aggregate state must equal CommandInFlight.

The correlation must equal the original pending correlation.

The deadline must equal the original pending deadline.

The DAG must report the prior Review phase/status despite durable Done bytes.

This proves plugin restart state comes from durable journal reality.

## Replay time

The tests should not use wall-clock sleeps.

Reconciliation time will be derived from the stored deadline.

One millisecond before that deadline is unambiguously inside the window.

The time is converted back to `SystemTime` for `dispatch_completion_at`.

This also proves the replay retains the original absolute bound.

## Convergence helper

A fixture method will run the transaction request again with the original key.

The returned commit ID must equal the first commit ID.

The returned committed-path list must be empty.

The repository commit count must remain baseline plus one.

The replay result is then delivered through `handle_completion_result`.

The aggregate must become Confirmed.

The confirmed commit ID must be the first commit ID.

The journal must contain one Requested, one CommandInFlight, one Confirmed.

The ledger must contain exactly one authoritative Done execution record.

The ticket's committed bytes must be Done.

This helper is used by both fixture tests.

## Restart reconstruction fixture

The first test starts from `LostResultFixture::new`.

It constructs a fresh adapter state from durable paths.

It checks exact in-flight reconstruction before any new observation.

It reconciles once and checks the replay pending record.

It checks the typed effect equals the original effect.

It converges through the real CLI transaction and adapter result boundary.

It then constructs another fresh state from the final journal.

That final restart must reconstruct Confirmed with the prior commit ID.

This makes the named terminal state durable across another plugin restart.

## Lost-result and duplicate-Stop fixture

The second test starts from the same lost-result prefix.

It reconstructs a fresh adapter state.

It submits duplicate Stop observations before replay.

Those observations must produce no effect and no pending invocation.

It reconciles once within the retained deadline.

It submits additional Stop and Reconcile observations while replay is pending.

Those observations must not launch a second effect.

They must not append journal transitions.

The test then converges on the first commit.

It delivers the same successful result again after confirmation.

The duplicate result must not add confirmation or provenance.

## Named bounded state interpretation

CommandInFlight is bounded by its stored reconciliation deadline.

For these success fixtures, its named terminal state is Confirmed.

The existing deadline regression separately proves the unsuccessful bound ends
in action-required Rejected.

The new fixtures should assert the retained deadline before replay and Confirmed
after replay, rather than duplicating the timeout-only test.

This matches the acceptance requirement that each fixture also yield Done.

## Non-goals

Do not change reducer behavior.

Do not change journal schema.

Do not change host-command construction.

Do not change CLI transaction identity.

Do not add live Zellij or provider execution.

Do not replace the broad hostile-order tests.

Do not remove older inline regressions in this ticket.

Removing duplicates can be evaluated separately after the story is complete.

## Verification decision

Run the focused module tests first.

Run the complete plugin native library next.

Run the entire workspace after the source commit.

Run `just check` for WASM checking plus workspace coverage.

Run formatting and diff hygiene checks.

Any production failure exposed by the new focused fixtures blocks disposition.
