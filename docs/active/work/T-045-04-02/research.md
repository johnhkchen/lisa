# Research — T-045-04-02 one-authoritative-completion

## Ticket boundary

The ticket asks for one authoritative completion across the Codex exit/revoke
boundary introduced by `T-045-04-01`.

The acceptance criterion is test-shaped.

It requires one ticket to move through claim, work, and the completion boundary.

The observable result must contain one completion record.

Repeated evidence must not inject a second completion effect.

Repeated result delivery must not publish a second completion.

Claude behavior must remain unchanged.

E-034 lease fencing must remain unchanged.

The parent story is `S-045-04`.

Its story acceptance joins four facts:

- the prior Codex TUI exits;
- the prior attempt is revoked;
- the next ticket launches in a fresh TUI;
- exactly one completion is recorded.

The story declares the proof boundary as fixture-based.

Live ticket-to-ticket field proof belongs to the next story.

## Repository organization

The relevant implementation is concentrated in
`crates/lisa-plugin/src/lib.rs`.

That file contains scheduler state, completion dispatch, pane lifecycle,
assignment claims, lease authority, provenance emission, and native tests.

Durable completion journal folding is in
`crates/lisa-plugin/src/completion_journal.rs`.

Pure completion state and reconciliation decisions are in
`crates/lisa-core/src/completion.rs`.

The host-side isolated completion transaction is in
`crates/lisa-cli/src/commit_transaction.rs`.

Cross-cutting hostile-order completion tests are in
`crates/lisa-plugin/src/tests/hostile_order_regression.rs`.

Provider-specific behavior is represented by `AgentClient` and adapter
resolution in the plugin.

## Assignment and claim lifecycle

`State::schedule_ready_tickets` selects ready tickets and eligible slots.

Dispatch mints an `AttemptLease` and retains it in `lease_high_water`.

The current authority is also installed in `current_leases`.

The lease is stamped into the thread and slot.

Codex gets a private assignment reference under the attempt work directory.

The reference includes the ticket, generation, nonce, and instructions path.

A fresh Codex launcher is sent to the pane.

The seat starts in a `Starting` state.

After bounded startup grace, the assignment enters `Delivering`.

An `AssignmentClaim` must match ticket ID, attempt generation, and nonce.

`State::admit_assignment_claim` promotes the seat to `Owned` only on an exact
match.

The lease and claim together form the Codex work authority.

Late or mismatched claims cannot restore ownership.

## Completion entry points

All production completion evidence enters `State::dispatch_completion`.

`CompletionInput` names the evidence origin.

The variants include artifact, stopped, idle, observed Done, operator request,
and reconciliation.

The dispatcher converts those inputs into the pure core completion reducer or
reconciler.

Only a returned `EffectCommand::LaunchCompletion` reaches the effect executor.

`State::execute_completion_effect` is the sole new-command launch boundary.

It validates that effect identity matches the source authority.

It rejects stale attempt leases.

It rejects dependency-blocked completion.

It derives the prior phase and status for later masking.

It creates a generation key from ticket, attempt, and generation number.

It persists `Requested` and `CommandInFlight` journal transitions.

It installs one `PendingCompletion` before launching the host command.

Native tests also append the inert effect to `launched_completion_effects`.

That vector is an observable count of completion command injections.

## Duplicate suppression

The pure reducer rejects another request while state is `Requested`,
`CommandInFlight`, or `Confirmed`.

The executor independently checks `pending_completions`.

It also checks durable aggregate state for requested and in-flight generations.

A confirmed aggregate suppresses another effect when the DAG ticket is durably
Done.

Artifact scanning can run repeatedly.

Review reconciliation can also run at each scheduler observation boundary.

Both paths converge on the same dispatcher and executor checks.

The production code therefore has layered idempotence boundaries.

The ticket asks for one combined regression across the new process boundary,
not a new completion gateway.

## Durable journal

`CompletionJournalTransition` has four states: requested, command-in-flight,
rejected, and confirmed.

The journal is append-only JSONL.

Every append reloads and folds the prior history before atomically publishing
the new bytes.

`CompletionJournalAggregate` retains the latest state for a ticket.

The aggregate key includes ticket ID, attempt ID, and generation.

`Confirmed` retains the commit ID.

Illegal transition sequences are rejected during folding.

While a transaction is uncertain, prior ticket phase/status mask Done bytes
written by the host command.

After confirmation, the durable Done state becomes visible to the DAG.

One successful ordinary transaction therefore has three records:

- one requested record;
- one command-in-flight record;
- one confirmed record.

Repeated result delivery has no pending entry and returns immediately.

It cannot append another confirmation.

## Result publication

`State::handle_completion_result` begins by cloning the pending record.

No pending record means the callback is ignored.

Attempt authority is revalidated before interpreting success.

A stale result is journaled as retryable rejection.

Success requires exit code zero and a hexadecimal 40- or 64-byte commit ID.

The ticket file is rescanned to verify durable Done frontmatter.

The confirmed journal transition is persisted before scheduler teardown.

The pending record is then removed.

The DAG is rebuilt from durable reality.

The thread is marked complete.

One authoritative `RunOutcome::Done` provenance record is emitted.

Only then is the completed slot released.

The thread is removed and ready tickets are scheduled.

## Provenance authority

`State::emit_provenance` reads the attempt lease stamped on the active thread.

For a Done outcome, the stamped lease must still match `current_leases`.

This check occurs before the completion release revokes the lease.

The resulting execution record has `authoritative: true`.

Release happens after emission, so the winning record can be written once.

After release the thread and current lease are absent.

A duplicate callback cannot reach provenance emission because pending state is
already absent.

The append-only provenance ledger is the externally inspectable completion
record used by existing tests.

## Codex completion boundary

`T-045-04-01` added `State::release_completed_slot_for_ticket`.

The helper snapshots whether the assigned pane hosts a live Codex TUI.

It calls the provider-neutral `release_slot_for_ticket` first.

Generic release revokes `current_leases[ticket]`.

It clears slot ticket and attempt lease.

It clears seat ownership.

It retains the lease high-water mark.

For the snapshotted Codex pane, the helper sends the adapter exit command.

It moves the unassigned slot to `WaitingForExit`.

It clears `has_session` and removes cooldown eligibility.

The slot cannot receive the successor during exit grace.

After the bounded grace expires, the slot represents an empty shell.

A later scheduling pass launches the successor through a fresh Codex launcher.

The successor gets a distinct assignment reference and nonce.

## Claude boundary

The completion helper snapshots only `last_client == Codex`.

Claude therefore takes the existing generic release behavior.

Its resident session and `/clear` reuse policy are unchanged.

The adapter interface was not changed by `T-045-04-01`.

Existing Claude reuse and acknowledgment tests exercise that policy.

This ticket does not need to put Claude through the Codex exit transition.

## E-034 lease boundary

Current authority remains represented by `current_leases`.

`lease_high_water` survives release so redispatch remains monotonic.

Thread and slot attempt stamps must agree with current authority.

Claim admission additionally requires the exact nonce-bearing assignment.

Completion dispatch rechecks current lease authority.

Done provenance rechecks current lease authority.

Release revokes authority before the pane becomes reusable.

No production lease type or admission predicate needs to change for this ticket.

## Existing tests and the uncovered seam

`codex_completion_exits_revokes_and_launches_next_fresh_tui` starts with real
scheduling and an exact Codex claim.

It proves revoke, exit grace, late-claim rejection, and fresh successor launch.

It currently simulates completion by writing Done, marking the thread complete,
and calling the release helper directly.

It does not enter completion dispatch, journal confirmation, or provenance.

`passing_review_hostile_order_converges_once_and_schedules_dependent` enters the
real isolated CLI completion transaction and proves idempotent replay.

It begins from a manually installed lease and slot rather than a claim.

It does not make the clean-exit state the center of its assertions.

`artifact_completion_publishes_only_after_verified_commit_result` proves that
verified success retires a Codex TUI.

It does not drive scheduling and claim acquisition.

Lease replacement tests prove one authoritative Done record after fencing.

Claude tests prove its acknowledgment and reuse path independently.

The missing assertion is one continuous fixture connecting the existing claim
boundary to the existing completion transaction and the new exit boundary.

## Baseline verification

The focused Codex boundary test passes before changes.

The hostile-order single-completion test passes before changes.

The focused Claude same-pane acknowledgment test passes before changes.

The attempt-lease focused test passes before changes.

The repository worktree contains unrelated Lisa runtime and planning files.

Those files are outside this ticket's ownership and must remain untouched.

The likely ticket-owned source surface is one native regression in
`crates/lisa-plugin/src/lib.rs`.
