# Structure: operator recovery test matrix

## Change set

This ticket changes test source only.

No production module, core reducer, UI renderer, or CLI behavior changes.

Two repository paths are ticket-owned source units.

## Modified file: `crates/lisa-plugin/src/lib.rs`

Add one child-module declaration inside the existing `#[cfg(test)] mod tests`.

The declaration is `mod operator_recovery_matrix;`.

Place it alongside the existing focused test module declarations.

The parent continues to own all production-private imports and common helpers.

No functions, structs, enums, or constants outside the test module change.

## Created file: `crates/lisa-plugin/src/tests/operator_recovery_matrix.rs`

This file contains the shared fixture and seven named tests.

It begins with `use super::*` to inherit the parent test module's private
surface.

It imports standard filesystem support locally.

The module is compiled only as part of native plugin tests through its parent.

## Constants

Define a single fixture ticket ID constant.

Define a stable pane ID for active-thread scenarios.

Keeping these values central prevents mismatched assertions.

## Fixture builder

`review_state()` returns `(State, tempfile::TempDir)`.

It creates temporary ticket and work directories.

It writes one canonical Review ticket.

It scans that ticket and constructs a `Dag`.

It initializes `PluginConfig.ticket_dir` and `PluginConfig.work_dir`.

It writes a canonical passing Review disposition.

It leaves thread, slot, lease, journal, and host execution state empty.

The retained `TempDir` owns all filesystem lifetime.

## Active attempt helper

`add_active_review_attempt(&mut State)` creates a running Review thread.

It creates one assigned agent slot for the same ticket and pane.

It calls the parent `install_current_attempt` helper.

The returned lease is the current attempt.

Thread and slot records receive the same lease through that helper.

## Stale record helper

The stale scenario first uses the active attempt helper.

It mints a checked successor from the returned lease.

It installs only that successor in current and high-water maps.

It intentionally leaves thread and slot records stamped with the predecessor.

This represents stale scheduler-adjacent attempt records with newer current
authority.

No production helper is changed to manufacture the case.

## Input helper

`submit_from_done_key(&mut State)` sends two `KeyWithModifier` values.

The first is bare character `d`.

The helper asserts modal open and correct ticket selection.

The second is Enter.

The helper asserts both events are handled.

Outcome-specific assertions remain in each test.

## Pending assertion helper

`assert_operator_pending(&State)` returns the correlation string.

It fetches the pending entry for the fixture ticket.

It checks operator authority and MarkDone source.

It checks the stable `operator` attempt identity.

It checks exactly one launch effect with the fixture completion ID.

It checks an open Pending modal with exact matching correlation.

Returning the correlation supports later duplicate and success assertions.

## Rejection assertion helper

`assert_named_rejection(&State, kind, detail_fragment)` returns correlation.

It locates the structured activity event for the fixture ticket and kind.

It checks non-empty correlation and detail content.

It checks modal Rejected state carries identical fields.

It checks the modal remains open for explicit acknowledgement.

The helper does not depend on activity log index.

## Test: active Review

Name the test for the matrix row.

Build the base fixture and add an active Review attempt.

Submit through the key helper.

Call the Pending helper.

Assert current lease, thread lease, and slot lease remain unchanged.

Assert the operator correlation does not use the numeric attempt identity.

## Test: orphaned Review

Build only the base fixture.

Assert no thread or current lease exists before input.

Submit through the key helper.

Call the Pending helper.

Assert the accepted request does not create attempt authority.

## Test: blocked disposition

Build the base fixture.

Overwrite canonical disposition with a valid Block document.

Submit through the key helper.

Assert no pending entry and no launch effect.

Call the rejection helper with `DispositionBlocked`.

Assert correlation is the stable operator generation.

## Test: stale attempt

Build the fixture and create stale thread/slot records.

Submit through the key helper.

Call the Pending helper.

Assert current authority remains the successor.

Assert stale thread and slot records remain the predecessor.

Assert no `StaleLease` activity was emitted for the operator request.

## Test: already pending

Build the base fixture.

Submit once and capture Pending correlation.

Open a new MarkDone modal while the pending transaction remains live.

Submit Enter again.

Call the rejection helper with `AlreadyPending`.

Assert rejected correlation equals the first Pending correlation.

Assert the launch-effect count remains one.

## Test: launch failure

Build the base fixture.

Set a non-empty completion journal path under the temp directory.

Keep `lisa_bin` absent.

Submit through the key helper.

Assert no pending entry and no launch effect.

Call the rejection helper with `LaunchFailed` and the missing binary detail.

Assert the correlation uses operator identity.

## Test: successful recovery

Build the fixture and active attempt.

Submit through the key helper.

Capture Pending correlation.

Update the real ticket path to Done.

Invoke `handle_completion_result` with exit zero and valid commit-shaped bytes.

Assert no pending entry.

Assert modal Accepted uses the captured correlation.

Assert the thread is removed and slot released.

Assert the DAG ticket has Done phase and status.

## Artifact files

The private attempt directory receives `research.md`, `design.md`,
`structure.md`, `plan.md`, `progress.md`, `review.md`, and
`review-disposition.json`.

These artifacts are not committed through the source-unit transaction.

Lisa owns their admission and final publication.

## Commit boundary

The meaningful source unit consists of the module registration and its new
test file.

Commit both exact paths together using `lisa commit-ticket`.

Do not include Lisa provenance, ticket frontmatter, private artifacts, or
unrelated untracked files.

## Verification boundary

First compile and run the focused matrix by test-name filtering.

Then run the complete `lisa-plugin` test target.

Then run workspace tests or the project `just check` gate.

Inspect final Git status to ensure both ticket-owned paths are committed.

Existing unrelated worktree entries must remain untouched.
