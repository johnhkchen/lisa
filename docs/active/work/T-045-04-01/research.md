# Research — T-045-04-01 clean-exit-revoke-attempt

## Ticket and story boundary

The ticket belongs to `S-045-04`, the clean ticket-boundary lifecycle slice.
Its direct dependencies, `T-045-02-02` and `T-045-03-03`, are complete.
The first dependency launches each Codex ticket through a Lisa-owned launcher.
The second dependency adds passive waiting for a delivered assignment claim.
This ticket covers the transition from one completed Codex ticket to the next.
The following ticket owns the exactly-one authoritative-completion assertion.
The later `S-045-05` story owns real Codex plus real Zellij field evidence.
This ticket is therefore a native scheduler fixture boundary, not a live test.

The acceptance criterion requires one observable sequence:

1. a ticket completes;
2. its Codex TUI receives a clean exit;
3. its attempt authority and assignment nonce become unusable;
4. a late claim for the predecessor is rejected;
5. the next ticket launches a fresh Codex TUI.

Claude behavior and hard-silence fencing are outside this change.

## Repository shape

The repository is a Rust workspace.
`lisa-core` owns provider-neutral tickets, claims, leases, and DAG types.
`lisa-cli` owns native claim and Codex process boundaries.
`lisa-plugin` owns scheduler state, Zellij pane input, and completion handling.
Most scheduler implementation and native tests live in one large file:
`crates/lisa-plugin/src/lib.rs`.
Provider-specific command policy lives in `crates/lisa-plugin/src/adapter.rs`.

The worktree contains unrelated runtime ledgers and materialized planning files.
The ordinary Git index is empty.
Ticket-owned source must be committed through `lisa commit-ticket` with exact paths.

## Attempt authority

`AttemptLease` is the provider-neutral authority token.
`State::current_leases` contains the currently authorized lease per ticket.
`State::lease_high_water` retains the last minted generation after revocation.
Removing an entry from `current_leases` revokes the active attempt.
Retaining the high-water entry makes a future attempt generation monotonic.

`State::revoke_current_lease` removes the current entry.
In native tests it appends `AttemptLifecycleEvent::LeaseRevoked`.
Repeated revocation is harmless because removal returns `None` thereafter.
The function does not delete the immutable assignment file.
Authority is revoked by making its lease non-current, not by erasing history.

`State::release_slot_for_ticket` starts by calling `revoke_current_lease`.
Its comment identifies release as the shared rescheduling boundary.
The method clears the slot's `ticket_id` and `attempt_lease`.
It removes the pane's `SeatAssignmentState`.
It removes human-attention state for the pane.
For a non-fenced live session it leaves `has_session` true.
It retains `last_client` and applies a wind-down cooldown.
It renames the pane to the provider-specific idle title.
In native tests it records `AttemptLifecycleEvent::SlotReleased`.

## Assignment nonce identity

`State::prepare_assignment` atomically publishes one assignment per attempt.
The resulting `AssignmentRef` carries the lease, nonce, and exact path.
`State::assignment_refs` retains the reference keyed by ticket ID.
The immutable file name includes both attempt generation and nonce.
Release currently leaves the historical `AssignmentRef` in this map.

`State::admit_assignment_claim` is the scheduler-side claim authority check.
It first requires a pane state with an active assignment generation.
It requires the slot ticket and slot lease to match the claim.
It requires the slot lease to be current in `current_leases`.
It then requires the retained assignment lease and nonce to match the claim.
Only after every check does it publish `SeatAssignmentState::Owned`.

After `release_slot_for_ticket`, a predecessor claim fails several checks:
the pane has no seat assignment, the slot has no ticket or lease, and the
predecessor lease is absent from `current_leases`.
The old assignment file and retained reference do not restore authority.
This is the existing nonce-revocation meaning used by the ticket.

The native `lisa claim-assignment` command also validates the pane lease marker.
It rejects stale attempts before publishing a claim signal.
The scheduler repeats current-lease validation when consuming that signal.
The ticket can exercise the scheduler admission boundary directly and cheaply.

## Codex process lifecycle

`CodexAdapter::reset_strategy` returns `ResetStrategy::ExitThenFresh`.
Its `exit_command` is the interactive `/exit` command.
Its fresh launch command invokes hidden `lisa launch-codex` plumbing.
The launch names the exact atomically published assignment path.
The native launcher starts a new interactive Codex child.

`AgentSlot::has_session` says a provider TUI is resident in the pane.
`AgentSlot::last_client` identifies the resident or incoming provider.
`TransitionState::WaitingForExit` prevents the slot from being selected.
`transition_started_at` starts the bounded exit grace.
After that grace, `check_transition_timeouts` handles `ExitReady`.

`check_transition_timeouts` already supports an unassigned exiting slot.
When `ticket_id` is absent, it changes the transition to `Idle`.
It clears `has_session` and `last_client`.
It removes any seat assignment and renames the pane to `lisa · idle`.
It does not launch a provider because no incoming ticket owns the slot.
This branch is covered by existing transition tests.

When `ticket_id` is present, the same timeout path launches the reserved ticket.
It republishes the exact assignment and lease marker.
It writes a fresh launch script and submits it to the pane.
It marks the slot as a live incoming provider session.
This is the current same-provider Codex reuse behavior.

## Current completion boundary

`State::handle_completion_result` is the verified completion-result boundary.
It requires a pending completion with current authority.
It requires a successful command result containing a commit ID.
It rescans durable ticket frontmatter and requires Done status and phase.
It persists the Confirmed completion journal transition before cleanup.
It logs the completed phase, Done transition, and verified commit.
It marks the thread complete and emits authoritative Done provenance.

After those durable steps it currently calls `release_slot_for_ticket`.
It then removes the thread and calls `schedule_ready_tickets` immediately.
Release revokes the completed attempt before scheduler admission resumes.
However, release deliberately leaves the resident Codex TUI alive.

`schedule_ready_tickets` sees the released Codex session as a compatible slot.
Because Codex uses `ExitThenFresh`, it treats that resident session as recycle.
It mints the next ticket's lease and publishes its assignment first.
It assigns the next ticket to the physical slot.
It sends `/exit` to the predecessor TUI.
It sets `WaitingForExit`, with the next ticket already reserved.
After the grace, `check_transition_timeouts` launches the fresh Codex process.

Thus current code eventually creates a fresh TUI when another ticket is ready.
The predecessor lease is revoked before the successor lease is minted.
A late predecessor claim is rejected by the existing admission checks.
But the clean exit is coupled to successor scheduling rather than completion.
With no immediately eligible successor, the completed Codex TUI stays resident.
With a successor, its lease and reservation exist while the old TUI exits.
The acceptance language requires exit to be a completion-boundary fact.

## Claude boundary

`ClaudeCodeAdapter::reset_strategy` is `ClearHandshake`.
Released Claude sessions are intentionally retained for `/clear` reuse.
The existing completion cleanup is shared by both providers.
Changing the generic release method would therefore alter Claude semantics.
The story explicitly says Claude need not share Codex's handshake.
A Codex-specific completion cleanup seam is required to preserve this boundary.

## Hard-silence and recovery boundaries

`revoke_and_fence_attempt` handles terminal hard-silent attempts.
It revokes authority, marks the pane `Fenced`, and closes the terminal pane.
Timeout and stale-thread paths call it before release.
Those paths intentionally do not perform graceful interactive exit.
They are separate from a successful completion boundary.

Startup recovery also uses `WaitingForExit`, but retains a ticket reservation.
It can revoke one failed attempt and install a successor in the same pane.
Its tests assert revoke, shell interruption, relaunch, and fence ordering.
This ticket must not weaken or redirect those existing recovery paths.

## Existing test infrastructure

Native plugin tests replace Zellij host calls with inert shims.
`send_line_to_pane` still queues a deferred Enter for every submitted line.
Tests observe activity events, pending Enter count, slot state, and written scripts.
`consecutive_reuse_state` constructs multiple Codex or Claude tickets and panes.
`active_ticket_panes` reports deterministic assignments.
`exit_then_deliver_fresh_codex` advances an exit and startup grace without sleeping.
`acknowledge_assignment` creates exact tagged hook evidence.

Claim tests construct `AssignmentClaim` from the retained assignment nonce.
They already prove wrong nonces and stale generations cannot own a seat.
Completion tests already exercise durable Done verification and slot release.
Transition tests already prove an unassigned `WaitingForExit` becomes a clean shell.
Consecutive reuse tests prove current Codex behavior sends `/exit` and launches fresh.

`AttemptLifecycleEvent` is a test-only safety-order trace.
It currently records lease revocation and slot release at normal release.
It records shell interruption/relaunch and hard fencing elsewhere.
It has no event for a successful completion's clean exit request.
`activity_log` records generic release and recycle messages.
It has no completion-specific Codex exit message today.

## Constraints and assumptions

The completion command and journal ordering must remain unchanged.
Attempt revocation must occur before the completed ticket becomes schedulable history.
The next ticket must not be minted into the pane before the old TUI exit grace ends.
No wall-clock sleep is needed; tests can inject an expired transition timestamp.
The immutable predecessor assignment may remain on disk for auditability.
Rejection must depend on current authority, not file deletion.
The outgoing `/exit` must use the resident Codex adapter's command.
The change must not affect Claude release and clear-handshake behavior.
Failure, timeout, reset, recovery, and fencing releases retain their current policy.
The source change is expected to remain localized to plugin scheduler code and tests.
