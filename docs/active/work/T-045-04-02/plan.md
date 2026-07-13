# Plan — T-045-04-02 one-authoritative-completion

## Objective

Turn the existing Codex exit/revoke boundary fixture into a continuous
claim→work→completion→exit→fresh-launch regression.

Prove that repeated completion evidence launches one completion effect.

Prove that repeated result delivery records one confirmed generation and one
authoritative Done provenance row.

Preserve Claude and E-034 lease behavior without production changes.

## Step 1 — enable durable observations in the boundary fixture

Modify only `crates/lisa-plugin/src/lib.rs`.

In `codex_completion_exits_revokes_and_launches_next_fresh_tui`, add fixture
paths for the completion journal and provenance ledger.

Install both paths in `State`.

Set `completion_journal_healthy` because the native fixture does not call load.

Keep all other scheduling configuration unchanged.

Verification:

- the test still schedules one predecessor into one empty pane;
- the fixture journal is writable;
- the fixture ledger is writable;
- no production field semantics change.

## Step 2 — retain exact scheduler claim acquisition

Keep the existing scheduler call.

Keep the startup grace transition.

Keep the exact assignment claim built from the scheduler reference.

Keep claim admission and `Owned` seat assertions.

Capture the scheduler-minted predecessor lease for later completion assertions.

Verification:

- the predecessor lease is current;
- the slot and thread carry that lease;
- the nonce-bearing claim is accepted;
- the pane is owned before Review artifacts appear.

## Step 3 — model completed work under the claimed lease

Advance the predecessor ticket file to Review through the ticket API.

Refresh the fixture DAG.

Advance the in-memory thread phase to Review.

Create the exact attempt-private work directory.

Write `review.md`.

Write a passing `review-disposition.json` using the existing helper.

Verification:

- the ticket is Review in the fixture DAG;
- the thread is Review;
- both artifacts exist under the claimed attempt;
- no canonical artifact is forged directly by the test.

## Step 4 — dispatch completion once

Run `check_artifact_advances`.

Confirm artifact admission creates a pending completion.

Clone its generation key and correlation data.

Assert its source is Artifact.

Assert its authority is the exact claimed lease.

Assert `launched_completion_effects` contains one effect.

Inspect the initial journal.

Assert one requested and one command-in-flight transition.

Verification:

- one typed effect is generated;
- the effect attempt matches the claimed generation;
- the completion ID matches the predecessor ticket;
- durable intent exists before Done publication.

## Step 5 — challenge pending duplicate suppression

Run artifact advancement again with the same files.

Call typed Review reconciliation with the same lease.

Expect no second dispatch.

Compare the journal to its pre-challenge bytes.

Assert the effect count remains one.

Verification:

- repeated artifact observation does not inject another command;
- reconciliation does not replay while the original pending entry exists;
- journal intent/in-flight transitions are not duplicated.

## Step 6 — publish one successful completion

Update the predecessor ticket file to durable Done.

Create a valid deterministic hexadecimal commit ID.

Call `handle_completion_result` with exit zero and that commit ID.

Allow the production result handler to:

- rescan durable Done;
- append confirmation;
- remove pending state;
- rebuild the DAG;
- mark the thread complete;
- emit Done provenance;
- revoke and release the slot;
- request clean Codex exit;
- remove the predecessor thread;
- attempt scheduling.

Verification:

- the pending entry is absent;
- the aggregate is Confirmed;
- the stored commit ID matches;
- predecessor authority is revoked;
- the predecessor thread is absent.

## Step 7 — challenge result idempotence

Read and retain the journal bytes after the first callback.

Read and retain the provenance bytes after the first callback.

Deliver the identical successful result a second time.

Read both files again.

Assert byte identity.

Verification:

- no second confirmation record appears;
- no second authoritative provenance row appears;
- no additional teardown action is performed;
- the completion effect count remains one.

## Step 8 — assert the one authoritative record

Parse the completion journal as text for stable transition counts.

Assert exactly three total lines.

Assert one requested state.

Assert one command-in-flight state.

Assert one confirmed state.

Parse provenance with the existing ledger helper.

Assert exactly one row.

Assert predecessor ticket ID.

Assert the claimed attempt lease.

Assert Done outcome.

Assert authoritative true.

Assert fenced false.

Verification:

- the ticket has one durable completion generation;
- the scheduler publishes one authoritative completion record.

## Step 9 — retain exit/revoke assertions

Assert the lifecycle trace is exactly revoke, release, clean exit.

Assert `current_leases` no longer contains the predecessor.

Assert `lease_high_water` retains it.

Assert the slot has no ticket or attempt lease.

Assert the seat is absent.

Assert `WaitingForExit` and no live session.

Assert the resident provider snapshot remains Codex during grace.

Assert completion-boundary activity was logged.

Verification:

- completion publication precedes revocation;
- revocation precedes possible reuse;
- process retirement is completion-owned.

## Step 10 — retain late-claim and reuse fencing

Retry the exact predecessor claim.

Assert it is rejected.

Call scheduling while exit grace is pending.

Assert no successor lease, assignment, thread, or launch is created.

Age the exit transition past its finite deadline.

Run timeout handling.

Assert the slot becomes an empty idle shell.

Verification:

- the predecessor nonce cannot regain authority;
- the successor cannot reserve the pane during exit;
- exit grace remains finite and deterministic.

## Step 11 — retain fresh successor launch

Schedule after shell readiness.

Capture the successor lease and assignment reference.

Assert the successor owns the pane reservation.

Assert a fresh Codex session is launched.

Assert the seat begins in `Starting` with the successor generation.

Assert predecessor and successor assignment paths differ.

Assert predecessor and successor nonces differ.

Assert the launcher script addresses the successor assignment.

Retry the predecessor claim again and assert rejection.

Verification:

- exactly-one completion does not prevent dependent scheduling;
- successor execution begins in a clean process with fresh identity.

## Step 12 — format and run focused tests

Run `cargo fmt --all -- --check` after formatting as necessary.

Run the strengthened boundary test with `--nocapture`.

Run the hostile-order convergence test.

Run completion-focused plugin tests.

Run the focused Claude same-pane acknowledgment test.

Run attempt-lease focused tests.

Run revoke-focused tests.

Verification:

- the new regression passes;
- existing real-CLI idempotence coverage remains green;
- Claude behavior remains green;
- E-034 lease behavior remains green.

## Step 13 — run the complete suite

Run `cargo test --workspace`.

If the workspace suite reveals an unrelated pre-existing failure, record exact
evidence in progress and review rather than changing unrelated files.

If the new test is flaky, remove wall-clock dependence by using the existing
injected deadlines rather than sleeps.

Verification:

- all workspace unit and integration tests pass;
- no production behavior regression is detected.

## Step 14 — record implementation progress

Create `progress.md` in the attempt work directory.

Record the source change.

Record the no-production-change decision.

Record focused and workspace test results.

Record any deviations from this plan before taking them.

Verification:

- progress accurately matches the final diff and commands.

## Step 15 — commit the ticket-owned source unit

Inspect `git diff -- crates/lisa-plugin/src/lib.rs`.

Run:

`lisa commit-ticket --ticket-id T-045-04-02 --message "test(plugin): prove one Codex boundary completion" --include crates/lisa-plugin/src/lib.rs`

Use no ordinary-index staging or commit command.

Verification:

- the command succeeds;
- the source file is clean afterward;
- unrelated worktree files remain untouched;
- the commit contains only the exact source path.

## Step 16 — Review

Write `review.md` in the attempt work directory.

Summarize the strengthened fixture and its durable assertions.

List the modified source file.

List test coverage, including Claude and lease regressions.

State that the host CLI commit layer remains covered by existing hostile-order
tests rather than duplicated in the boundary fixture.

Identify any open concerns.

Write exact passing disposition JSON only if all required checks pass and the
ticket-owned source is clean.

Remain on this ticket after Review.
