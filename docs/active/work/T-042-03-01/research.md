# Research: operator-requested authority emission

## Ticket boundary

T-042-03-01 is the first ticket in Story S-042-03.
The story makes dashboard recovery a first-class operator command.
This ticket owns the authority-emission decision only.
Later tickets own modal persistence, durable confirmation, and the full matrix.
The acceptance criterion focuses on the `[d]one` path.
That path must work with or without a live thread.
It must never use `CompletionAuthority::Attempt`.
It must carry a source that can be inspected and audited.
It must retain Review-disposition and dependency refusal.

## Repository and module layout

The relevant production code is in `crates/lisa-plugin/src/lib.rs`.
The plugin keeps scheduler state, modal state, and completion adapter state there.
The pure completion contract is in `crates/lisa-core/src/completion.rs`.
The E-040 disposition vocabulary and parser are in lisa-core.
Ticket dependency state is represented by `lisa_core::dag::Dag`.
The UI renderer is in `crates/lisa-plugin/src/ui.rs`.
This ticket does not require a renderer change.

## Keyboard and modal path

`State::handle_key` handles normal dashboard input.
The bare `d` key calls `State::open_mark_done_modal`.
The modal contains sorted ticket IDs and a cursor.
Pressing Enter in `ModalMode::MarkDone` calls `State::mark_ticket_done`.
The handler currently closes the modal after dispatch.
Modal retention belongs to T-042-03-02 rather than this ticket.

`open_mark_done_modal` filters Done tickets.
It normally excludes tickets with running threads.
It explicitly includes Review tickets even when their thread is running.
It also includes Implement tickets with a `review.md` artifact.
Consequently `mark_ticket_done` must correctly handle active and orphaned tickets.

## Existing completion adapter

`CompletionInput` is the plugin's typed adapter input enum.
Artifact, Reconcile, Stopped, Idle, and ObservedDone are distinct variants.
The operator path is currently named `Manual`.
Its fields are a ticket ID and `Option<CompletionAuthority>`.
This makes the input shape ambiguous about who requested completion.
It can carry attempt authority, operator authority, or no authority.

`CompletionAuthority` has two variants.
`Attempt` owns an `AttemptLease`.
`Operator` has no attempt lease.
`PendingCompletion` retains the chosen authority and `CompletionSource`.

`CompletionSource` currently uses a unit `Manual` variant.
Other variants describe artifact, reconcile, idle, stop, and observed-Done origins.
The unit Manual source distinguishes manual completion from scheduler completion.
It does not identify which operator surface emitted the request.

## Current authority selection

`State::mark_ticket_done` examines `self.threads`.
If a thread exists, it takes the thread's optional `attempt_lease`.
A present lease becomes `CompletionAuthority::Attempt`.
If the thread has no lease, the authority remains absent.
If there is no thread, the method creates `CompletionAuthority::Operator`.
Thus authority depends on incidental thread presence.
The same `[d]one` gesture can represent two different principals.
An active Review normally causes the operator gesture to borrow attempt authority.
An orphaned Review uses operator authority.

## Dispatch behavior

`State::dispatch_completion` is the sole typed completion adapter.
The Reconcile branch uses durable inputs and the pure reconciler.
Other branches normalize input into ticket, source, authority, and review lease.
Artifact, Stopped, and Idle carry attempt authority and correlated review admission.
ObservedDone optionally carries an attempt lease.
Manual passes through its supplied optional authority.

The adapter derives an `AttemptId` for the pure reducer event.
Attempt authority uses the numeric attempt ID.
Operator authority uses the stable string `operator`.
Missing authority uses `missing-authority` and later fails executor validation.
The reducer receives `CompletionEvent::Request`.
The reducer returns an inert `EffectCommand::LaunchCompletion` on acceptance.
Only `execute_completion_effect` launches the command.

## Effect executor authority checks

`execute_completion_effect` checks effect identity against source authority.
Attempt effects must match their lease's attempt ID.
Operator effects pass the attempt-ID comparison because their identity is adapter-defined.
The executor then suppresses pending or confirmed duplicates.
It accepts a current attempt lease.
It accepts operator authority only when the source is Manual.
Stale attempt leases are rejected and logged.
Missing or mismatched authority is rejected.

The executor calls `Dag::all_dependencies_done`.
Unmet dependencies produce `CompletionRejection::DependencyBlocked`.
The rejection is converted to a correlated activity event.
No pending completion or command effect is recorded for a dependency refusal.

## Review disposition gate

E-040 uses `review-disposition.json` as the explicit Review verdict.
`ReviewDisposition` can be Pass, Block with a reason, or Invalid with a reason.
`State::admit_passing_review` admits the artifact and parses the canonical copy.
Block and Invalid become `CompletionRejection::DispositionBlocked`.
Missing or unadmittable disposition evidence also fails closed.

Attempt-driven Artifact, Stopped, and Idle inputs call `admit_correlated_review`.
That method validates the current lease and calls `admit_passing_review`.
Reconciliation separately builds `DurableCompletionInputs` and uses the pure reconciler.
The current Manual branch supplies no review lease.
It therefore skips `admit_correlated_review`.
The executor checks dependencies but does not currently parse disposition.
This means the existing manual path can bypass a blocking disposition.

## Artifact admission constraints

Leased attempts write private artifacts below `.lisa/attempts`.
`admit_artifact` verifies the exact current lease before publishing canonical bytes.
The unleased branch only accepts canonical artifacts when no current lease exists.
That fallback exists for historical fixtures without registered authority.
An operator request must not publish an active attempt's private artifact.
Doing so would implicitly reuse attempt authority.
For an active Review, the operator can consume the already-admitted canonical verdict.
The canonical verdict is the E-040 decision visible to completion publication.

## Pure reducer boundary

The lisa-core reducer intentionally has no scheduler or filesystem dependencies.
Its `CompletionEvent::Request` carries an AttemptId and CompletionId.
The reducer contract does not define operator authority.
Story S-042-03 explicitly excludes changes to the E-041 reducer contract.
The plugin adapter is therefore responsible for mapping operator identity to the reducer.
The stable `operator` AttemptId is already the mapping used by production code.

The reducer rejects duplicate requests based on aggregate state.
Disposition eligibility is handled by durable reconciliation or adapter admission.
Dependency state is plugin DAG state and is checked at the executor boundary.
This ticket must preserve those existing architectural boundaries.

## Tests and fixtures

Native plugin tests are embedded in `crates/lisa-plugin/src/lib.rs`.
`test_mark_done_keeps_thread_and_slot_until_commit_result` covers active-thread state.
It currently expects an attempt-authorized effect with attempt ID `1`.
That expectation directly records the behavior this ticket changes.
It also verifies the thread and slot remain until completion succeeds.

`test_mark_done_without_active_attempt_uses_operator_authority` covers an orphaned Review.
It expects operator authority and reducer identity `operator`.
It confirms the ticket remains Review while the command is pending.

Disposition tests already cover automatic completion refusal.
Dependency tests already cover the executor gate in other completion paths.
No existing test couples `[d]one` to both gates.
The ticket requires a focused operator-path regression.

## Working-tree constraints

The repository uses concurrent ticket work on one branch.
Unrelated changes exist in provenance and ticket/work metadata.
Ticket-owned source work is limited to `crates/lisa-plugin/src/lib.rs`.
Phase artifacts belong only in this attempt-private work directory.
Source changes must be committed with `lisa commit-ticket` and an exact include path.
The ticket frontmatter is controlled by Lisa and must not be edited manually.

## Research conclusions

The defect is localized to the shape and construction of the manual adapter input.
Thread presence currently changes the authority of the same operator action.
The adapter already has a stable operator reducer identity.
The executor already has a dependency refusal boundary.
Canonical E-040 disposition parsing already exists after artifact admission.
The implementation must distinguish operator provenance without changing lisa-core.
The active-thread state-retention behavior is independent and should remain intact.
The later modal and full-matrix tickets remain outside this ticket's scope.
