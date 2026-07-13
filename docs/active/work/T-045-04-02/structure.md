# Structure — T-045-04-02 one-authoritative-completion

## Change surface

One repository source file will be modified:

`crates/lisa-plugin/src/lib.rs`

No production module will be created, modified, or deleted.

No public interface will change.

No core completion type will change.

No completion journal schema will change.

No CLI transaction behavior will change.

No adapter behavior will change.

No lease representation will change.

## Test location

The existing native test
`codex_completion_exits_revokes_and_launches_next_fresh_tui` remains in the
`#[cfg(test)] mod tests` section of `crates/lisa-plugin/src/lib.rs`.

It is adjacent to provider scheduling and consecutive-reuse fixtures.

That placement gives it direct access to private scheduler state.

It can observe `pending_completions`.

It can observe `completion_aggregates`.

It can observe `launched_completion_effects`.

It can observe the test-only `attempt_lifecycle` trace.

It can invoke private claim and completion methods.

No visibility widening is required.

## Fixture configuration changes

The existing temp directory remains the filesystem root for the test.

The ticket directory remains `temp/tickets`.

The canonical work directory remains `temp/work`.

The private attempt directory remains `temp/attempts`.

The signal directory remains `temp/signals`.

Add a completion journal path at `temp/completion-journal.jsonl`.

Add a provenance ledger path at `temp/provenance.jsonl`.

Set `completion_journal_healthy` to true in fixture state.

The test does not invoke plugin load, so health must be explicit.

The configured Lisa binary remains a fixture path.

Native host calls remain no-op stubs.

## Existing claim section

Retain predecessor scheduling through `schedule_ready_tickets`.

Retain extraction of the scheduler-minted predecessor lease.

Retain extraction of the exact assignment reference.

Retain deterministic startup grace advancement.

Retain construction of `AssignmentClaim` from ticket, generation, and nonce.

Retain admission of the claim.

Retain the assertion that the seat is `Owned`.

This section establishes the authoritative attempt for later completion.

## New work section

Insert a work-completion section after the exact claim.

Use the ticket API to update the predecessor phase to Review.

Refresh the DAG from the ticket directory.

Set the predecessor thread's current phase to Review.

Resolve the private work directory with `attempt_work_dir`.

Create the directory.

Write a minimal `review.md` artifact.

Write the exact passing `review-disposition.json` artifact through the existing
test helper.

This section represents completed agent work under the claimed lease.

## New completion dispatch section

Call `check_artifact_advances`.

Assert that Review is admitted and one completion becomes pending.

Clone the pending completion for stable identity assertions.

Assert its source is `CompletionSource::Artifact`.

Assert its authority is the predecessor attempt lease.

Assert exactly one `EffectCommand::LaunchCompletion` was recorded.

Read the journal and assert the initial two transition records.

Call `check_artifact_advances` a second time.

Call `dispatch_completion(CompletionInput::Reconcile)` with the same lease.

Assert the reconciliation call produces no new dispatch.

Assert the effect count and journal bytes remain unchanged.

This is the no-double-inject proof.

## New result section

Update predecessor ticket frontmatter to Done.

Construct one valid hexadecimal commit ID byte vector.

Call `handle_completion_result` once.

Snapshot the confirmed journal bytes.

Snapshot the provenance ledger bytes.

Call `handle_completion_result` a second time with the same result.

Assert both snapshots remain byte-identical.

This is the no-second-completion proof.

## New journal assertions

Assert `pending_completions` no longer contains the predecessor.

Assert `completion_aggregates[predecessor]` is `Confirmed`.

Assert its confirmed commit ID matches the fixture commit ID.

Assert the journal contains exactly three lines.

Assert exactly one line is requested.

Assert exactly one line is command-in-flight.

Assert exactly one line is confirmed.

The journal assertions prove one durable generation lifecycle.

## New provenance assertions

Read the ledger with the existing `read_ledger` test helper.

Assert it has exactly one row.

Assert the row's ticket ID is the predecessor.

Assert the row's attempt lease equals the claimed lease.

Assert its outcome is `RunOutcome::Done`.

Assert `authoritative` is true.

Assert `fenced` is false.

The provenance row is the single authoritative completion record.

## Completion boundary replacement

Delete the existing direct completion simulation:

- direct `update_ticket_done` followed by fixture DAG refresh;
- direct `thread.complete()`;
- direct `release_completed_slot_for_ticket`;
- direct removal of the predecessor thread.

Those actions are now performed by `handle_completion_result` after durable
success verification.

The existing thread-completed assertion moves to post-result absence and record
assertions.

This replacement is the structural center of the ticket.

## Retained process-boundary section

Retain the ordered lifecycle trace assertion.

The expected events remain:

- `LeaseRevoked`;
- `SlotReleased`;
- `CleanExitRequested`.

Retain current-lease absence.

Retain lease high-water preservation.

Retain cleared slot ticket and attempt lease.

Retain `WaitingForExit`.

Retain no live session and Codex resident identity.

Retain seat assignment absence.

Retain the completion-boundary activity log assertion.

These assertions now follow the real result publisher.

## Retained late-claim section

Retain rejection of the exact predecessor claim immediately after completion.

Retain the assertion that the seat remains unassigned.

Retain scheduling while exit is pending.

Retain absence of successor lease, assignment reference, and thread.

Retain the successor launch-count assertion.

This section connects completion publication to lease revocation.

## Retained shell and successor section

Retain injected aging of `transition_started_at`.

Retain `check_transition_timeouts`.

Retain the empty idle shell assertions.

Retain scheduling of the successor.

Retain successor slot, lease, session, provider, and starting-seat assertions.

Retain assignment path and nonce inequality.

Retain fresh launch script assertions.

Retain rejection of the predecessor claim after successor launch.

This section proves the new completion record did not weaken physical isolation.

## Imports and helper reuse

The test already imports `std::fs`.

Remove the now-unused local `ThreadStatus` import.

Use `RunOutcome` already available to the enclosing test module through
`super::*` imports.

Use the existing `write_passing_review_disposition` helper.

Use the existing `read_ledger` helper.

Use the existing `refresh_fixture_dag` helper.

No new general-purpose helper is expected.

## Artifact files

Attempt-private RDSPI artifacts live under
`.lisa/attempts/T-045-04-02/1/work/`.

They are not ticket-owned source changes for the isolated source commit.

`research.md`, `design.md`, `structure.md`, and `plan.md` precede implementation.

`progress.md` records implementation and verification.

`review.md` and `review-disposition.json` complete the handoff.

Lisa will admit and publish them after lease verification.

## Commit boundary

The only ticket-owned source unit is
`crates/lisa-plugin/src/lib.rs`.

Commit it with one exact include path through `lisa commit-ticket`.

Do not include runtime `.lisa` ledgers or unrelated planning files.

Do not use the ordinary Git index.

After the isolated commit, the source file must be clean.

## Verification boundaries

Run the strengthened test with `--nocapture` to retain its transcript.

Run completion-focused plugin tests.

Run the prior Codex and hostile-order regression by focused names as useful.

Run the focused Claude same-pane test.

Run attempt-lease and revocation-focused tests.

Run the full workspace suite.

Run formatting verification.

Inspect Git status to confirm only unrelated pre-existing files and attempt
artifacts remain outside the isolated source commit.
