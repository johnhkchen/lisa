# Review — T-045-04-02 one-authoritative-completion

## Outcome

The ticket is ready to complete.

The Codex boundary regression now proves one continuous
claim→work→completion→exit/revoke→fresh-launch lifecycle.

Repeated Review evidence produces one completion effect.

Repeated successful result delivery produces one confirmed journal transition
and one authoritative Done provenance record.

The existing clean-exit and lease-revocation assertions remain intact.

No production implementation changed.

No Claude behavior changed.

No E-034 lease behavior changed.

## Commit

Ticket-owned source was committed through Lisa's isolated transaction.

Commit:

`38e0fa2d69f7a43e2764a7fe0fe75e2858c3d624`

Message:

`test(plugin): prove one Codex boundary completion`

The commit contains one exact path:

`crates/lisa-plugin/src/lib.rs`

No ordinary-index staging or ordinary Git commit was used for ticket work.

The source path is clean after the isolated commit.

## Modified file

### `crates/lisa-plugin/src/lib.rs`

Strengthened the existing native test
`codex_completion_exits_revokes_and_launches_next_fresh_tui`.

The test previously scheduled and claimed the predecessor, then simulated the
boundary by directly marking Done, completing the thread, and calling the
release helper.

It now reaches the same boundary through typed artifact completion and the
production result publisher.

The source diff contains 114 insertions and 6 deletions.

All changes are inside the native test module.

## Claimed authority setup

The fixture still begins with a ready Codex predecessor and dependent successor.

It still has one physical pane and a concurrency cap of one.

The scheduler mints the predecessor attempt lease.

The scheduler writes an attempt-specific assignment reference.

The fixture advances the bounded startup grace into assignment delivery.

It constructs an exact `AssignmentClaim` from:

- predecessor ticket ID;
- scheduler-minted attempt generation;
- scheduler-retained assignment nonce.

The claim is admitted and the seat becomes `Owned`.

This is important because the completion is now tied to authority acquired
through the real Codex claim path.

## Work and Review setup

After ownership, the fixture moves the predecessor ticket and thread to Review.

It writes `review.md` inside the exact attempt-private work directory.

It writes the exact passing disposition JSON through the existing helper.

It calls normal artifact advancement.

Artifact admission copies the current attempt's Review material to canonical
work and enters `CompletionInput::Artifact`.

The resulting pending transaction carries the claimed attempt lease.

## No-double-inject assertion

The test inspects the test-only `launched_completion_effects` vector.

After initial artifact admission it contains exactly one
`EffectCommand::LaunchCompletion`.

The effect's attempt ID equals the claimed generation.

The effect's completion ID equals the predecessor ticket.

The fixture journal contains exactly:

- one requested transition;
- one command-in-flight transition.

The test then runs artifact advancement again.

It also requests typed Review reconciliation with the same current lease.

Neither path launches another effect while the original transaction is pending.

The effect count remains one.

The journal remains byte-identical.

This directly covers the acceptance requirement that completion not be injected
twice at the boundary.

## One durable confirmation

The test updates the predecessor ticket to durable Done to represent the
successful isolated host transaction's filesystem effect.

It delivers a valid successful result through `handle_completion_result`.

The normal result publisher verifies durable Done.

It appends one confirmed transition.

It removes pending state.

It rebuilds the DAG.

It emits authoritative provenance.

It performs successful completion release.

It removes the predecessor thread.

The aggregate is asserted to be `CompletionState::Confirmed`.

Its generation key equals the original pending generation.

Its confirmed commit ID equals the delivered fixture commit ID.

## No-second-completion assertion

After the first result, the test snapshots journal and provenance bytes.

It delivers the identical successful result a second time.

No pending transaction remains, so result handling is inert.

The journal remains byte-identical.

The provenance ledger remains byte-identical.

The completion effect count remains one.

The final journal contains exactly three records:

- one requested;
- one command-in-flight;
- one confirmed.

No second confirmation exists.

## One authoritative completion record

The provenance ledger is parsed through the established test helper.

It contains exactly one record.

That record names the predecessor ticket.

It carries the exact claimed attempt lease.

Its outcome is `RunOutcome::Done`.

Its `authoritative` field is true.

Its `fenced` field is false.

The test prints a stable summary row:

`T0450402|completion|ticket=T-BOUNDARY-01|effects=1|confirmed=1|authoritative=1|duplicate_result=ignored`

This row makes the acceptance facts visible in the focused transcript.

## Completion boundary coverage

The strengthened test retains the exact lifecycle trace assertion.

The successful result produces, in order:

1. lease revocation;
2. slot release;
3. clean Codex exit request.

The predecessor is absent from `current_leases`.

Its lease remains in `lease_high_water`.

The slot has no ticket ID or attempt lease.

The seat assignment is removed.

The pane is `WaitingForExit` with no live session.

The resident-provider snapshot remains Codex until exit grace resolves.

The completion-boundary activity entry is present.

## Late claim and fresh launch coverage

The exact predecessor claim is rejected immediately after completion.

Scheduling during exit grace cannot mint or reserve the successor.

The test advances exit grace with an injected timestamp rather than sleeping.

Timeout handling publishes an empty idle shell.

The next scheduling pass launches the dependent successor.

The successor receives a fresh attempt lease.

It receives a distinct assignment path.

It receives a distinct nonce.

Its fresh launcher points at its own assignment reference.

The predecessor claim remains rejected after successor launch.

## Existing real transaction coverage

The strengthened fixture does not duplicate temporary Git repository setup.

It models successful host completion with Done frontmatter and a valid result
callback.

The actual isolated `complete_ticket` transaction remains covered by
`hostile_order_regression`.

That suite asserts same-generation replay discovers the prior commit rather than
creating a second completion commit.

It also asserts one confirmation and one authoritative provenance record across
lost-result and duplicate-stop orders.

Together, the existing host transaction tests and the strengthened boundary
test cover both durable commit idempotence and the physical Codex handoff.

## Claude preservation

No production adapter or completion-release code changed.

The Codex-only predicate in `release_completed_slot_for_ticket` is unchanged.

Claude continues to use its existing SessionStart and `/clear` reuse behavior.

The focused test
`same_pane_replacement_requires_start_and_chat_ack_for_claude` passes.

The workspace suite also passes all Claude-related tests.

## E-034 lease preservation

No `AttemptLease` field or method changed.

No current-lease predicate changed.

No claim admission predicate changed.

No seat ownership transition changed.

No revocation method changed.

No provenance authority predicate changed.

The attempt-lease focused test passes.

The revoke-focused test passes.

The full workspace suite passes lease and fencing regressions.

## Verification performed

Focused strengthened test:

`cargo test -p lisa-plugin codex_completion_exits_revokes_and_launches_next_fresh_tui -- --nocapture`

Result: 1 passed, 0 failed.

Completion-focused plugin tests:

`cargo test -p lisa-plugin completion`

Result: 22 passed, 0 failed.

Claude regression:

`cargo test -p lisa-plugin same_pane_replacement_requires_start_and_chat_ack_for_claude`

Result: 1 passed, 0 failed.

Lease regression:

`cargo test -p lisa-plugin attempt_lease`

Result: 1 passed, 0 failed.

Revocation regression:

`cargo test -p lisa-plugin revoke`

Result: 1 passed, 0 failed.

Full verification:

`cargo test --workspace`

Result: passed, including 395 plugin tests, 19 CLI library tests, the CLI/core
suites, and doc tests.

Formatting and whitespace:

`cargo fmt --all -- --check`

`git diff --check`

Both passed.

## Worktree review

The committed source path is clean.

The isolated commit contains no artifact or runtime ledger path.

The shared worktree still contains unrelated pre-existing Lisa runtime and
planning material.

Those files were preserved and excluded from the ticket source commit.

Attempt-private phase artifacts are complete through Review.

Lisa owns their admission and final completion publication.

## Open concerns and limitations

No critical issue blocks completion.

The combined test uses native no-op Zellij host bindings, as do adjacent
scheduler lifecycle fixtures.

It proves state, journal, provenance, command-effect count, and transcript
ordering rather than launching a real interactive Codex process.

That is consistent with the story's declared fixture-proven boundary.

The live ticket-to-ticket handoff remains assigned to the later field-test
story.

The completion journal and provenance paths are test-local and removed with the
temporary directory.

No schema migration or runtime compatibility concern is introduced.

## Disposition

Pass.

The acceptance criterion is covered by one continuous regression.

The source is committed through the required isolated transaction.

All focused and workspace verification is green.
