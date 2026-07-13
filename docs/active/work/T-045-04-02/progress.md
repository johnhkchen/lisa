# Progress — T-045-04-02 one-authoritative-completion

## Status

Implementation is complete.

The ticket-owned source change is limited to
`crates/lisa-plugin/src/lib.rs`.

No production path changed.

The existing Codex boundary regression now drives the claimed attempt through
typed completion and durable result publication before testing exit/revocation.

All planned focused and workspace checks pass.

The source unit is ready for the required isolated Lisa commit.

## Completed — repository and boundary research

Read `CLAUDE.md` and the injected assignment.

Read `docs/active/tickets/T-045-04-02.md`.

Read `docs/knowledge/rdspi-workflow.md`.

Read the parent story `docs/active/stories/S-045-04.md`.

Read the predecessor ticket and admitted work for `T-045-04-01`.

Mapped typed completion dispatch and execution in the plugin.

Mapped durable completion journal reconstruction and folding.

Mapped current-lease, high-water, claim, seat, and provenance checks.

Mapped the successful Codex completion release helper.

Compared the claim/exit regression with hostile-order completion regressions.

Identified the missing combined claim→completion→exit seam.

## Completed — RDSPI artifacts

Wrote attempt-private `research.md`.

Wrote attempt-private `design.md`.

Wrote attempt-private `structure.md`.

Wrote attempt-private `plan.md`.

The artifacts document the test-only decision and exact source boundary.

No artifact was written directly to the shared active work directory.

Lisa may admit attempt artifacts independently as part of its normal workflow.

## Completed — durable fixture configuration

Added a temporary completion journal path to the existing boundary test.

Added a temporary provenance ledger path.

Configured the temp directory as project and Git root for completion argv
construction.

Marked the journal healthy because native fixture construction does not execute
plugin load.

Kept the existing one-pane Codex scheduling configuration.

Kept zero wind-down and deterministic assignment acknowledgment timing.

## Completed — exact claim acquisition

Retained scheduling of the predecessor from a ready DAG.

Retained scheduler minting of the predecessor attempt lease.

Retained creation of the private assignment reference.

Retained startup-grace advancement into assignment delivery.

Retained the exact nonce-bearing `AssignmentClaim`.

Retained claim admission into `SeatAssignmentState::Owned`.

The completion assertions therefore use authority acquired through the real
claim path rather than a manually installed test lease.

## Completed — work and Review admission

The test advances the predecessor ticket to Review.

It refreshes the fixture DAG.

It sets the claimed running thread to Review.

It writes `review.md` under the exact attempt-private work directory.

It writes the exact passing `review-disposition.json` through the existing
helper.

It invokes normal artifact advancement.

The admitted Review dispatches completion through `CompletionInput::Artifact`.

## Completed — no-double-inject proof

The test asserts the pending source is Artifact.

It asserts pending authority equals the exact claimed attempt lease.

It asserts one `EffectCommand::LaunchCompletion` with matching ticket and
attempt identity.

It reads the in-flight journal.

It asserts one requested record and one command-in-flight record.

It runs artifact advancement again.

It runs typed Review reconciliation with the same lease.

It asserts reconciliation does not dispatch.

It asserts the effect count remains one.

It asserts the journal remains byte-identical.

This directly covers the acceptance criterion's no-double-inject clause.

## Completed — one result publication

The test writes durable Done frontmatter to model successful completion command
effects.

It delivers one valid hexadecimal commit ID through
`handle_completion_result`.

The production result handler verifies Done.

It appends the confirmed transition.

It removes pending state.

It rebuilds the DAG.

It emits authoritative Done provenance.

It revokes and releases the claimed slot.

It requests clean Codex exit.

It removes the predecessor thread.

## Completed — duplicate-result proof

The test snapshots confirmed journal bytes.

It snapshots provenance ledger bytes.

It delivers the identical successful result a second time.

It asserts the journal remains byte-identical.

It asserts provenance remains byte-identical.

It asserts the effect count remains one.

It asserts the journal contains exactly three total transitions.

It asserts one requested, one command-in-flight, and one confirmed record.

It asserts the aggregate is Confirmed with the expected commit ID.

## Completed — authoritative completion record

The provenance ledger is parsed with the existing helper.

The test asserts exactly one record.

The record names the predecessor ticket.

The record carries the claimed attempt lease.

The outcome is Done.

The record is authoritative.

The record is not fenced.

The fixture prints a stable `T0450402|completion` transcript row showing one
effect, one confirmation, one authoritative record, and ignored duplicate
result delivery.

## Completed — exit/revoke/fresh launch

Retained exact lifecycle ordering:

- lease revoked;
- slot released;
- clean exit requested.

Retained current-lease absence.

Retained lease high-water preservation.

Retained cleared slot and seat state.

Retained `WaitingForExit` and no live session.

Retained rejection of the exact predecessor claim after completion.

Retained suppression of successor scheduling while exit is pending.

Retained injected exit-grace expiration.

Retained empty-shell publication.

Retained fresh successor Codex launch with a distinct assignment path and nonce.

Retained predecessor claim rejection after the successor launch.

## Plan deviations

No material deviation from the selected design occurred.

The implementation reused the existing boundary test rather than creating a new
fixture, as planned.

The real CLI `complete_ticket` call remains in hostile-order coverage.

The boundary test models its successful durable effects with Done frontmatter
and a valid callback, as designed.

No production latch or state field was added.

No helper extraction was needed.

## Baseline verification

Before implementation, the following passed:

`cargo test -p lisa-plugin codex_completion_exits_revokes_and_launches_next_fresh_tui -- --nocapture`

`cargo test -p lisa-plugin passing_review_hostile_order_converges_once_and_schedules_dependent -- --nocapture`

`cargo test -p lisa-plugin same_pane_replacement_requires_start_and_chat_ack_for_claude`

`cargo test -p lisa-plugin attempt_lease`

## Post-implementation focused verification

The strengthened regression passed with `--nocapture`.

Its transcript reported:

`T0450402|completion|ticket=T-BOUNDARY-01|effects=1|confirmed=1|authoritative=1|duplicate_result=ignored`

Completion-focused plugin tests passed: 22 passed, 0 failed.

The focused Claude same-pane replacement test passed.

The attempt-lease focused test passed.

The revoke-focused test passed.

The hostile-order real transaction test remained included in focused/full
verification and passed.

## Full verification

`cargo test --workspace` passed.

The plugin test binary reported 395 passed, 0 failed.

The CLI library test binary reported 19 passed, 0 failed.

The CLI binary suite and core suite passed within the workspace command.

All doc tests passed.

`cargo fmt --all -- --check` passed.

`git diff --check` passed.

## Worktree ownership

The source diff is 114 insertions and 6 deletions in one test.

Unrelated runtime and planning files were already modified or untracked in the
shared worktree.

They include Lisa provenance/completion runtime files and epic/story/ticket
materialization.

They were not edited as part of this ticket's source implementation.

They will not be included in the isolated source commit.

## Isolated source commit

Committed the exact ticket-owned source path with:

`lisa commit-ticket --ticket-id T-045-04-02 --message "test(plugin): prove one Codex boundary completion" --include crates/lisa-plugin/src/lib.rs`

The command returned commit
`38e0fa2d69f7a43e2764a7fe0fe75e2858c3d624`.

`git show` confirms the commit contains only
`crates/lisa-plugin/src/lib.rs`.

The source path has no remaining staged, modified, or untracked change.

Unrelated shared-worktree files remain outside the commit.

## Remaining action

Write `review.md` and exact `review-disposition.json`, then remain on this ticket
for Lisa's completion admission.
