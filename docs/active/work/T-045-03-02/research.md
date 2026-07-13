# Research — T-045-03-02 evidence tiers: hook and artifact

## Ticket boundary

T-045-03-02 is the middle ticket in story S-045-03.

The preceding ticket, T-045-03-01, made an exact agent-issued claim sufficient
to establish scheduler ownership.

The following ticket, T-045-03-03, owns the new delivered-awaiting-claim state,
the no-reinjection behavior, and its timeout resolution.

This ticket is limited to the evidence relationship among:

- the authoritative exact claim;
- a matching `UserPromptSubmit` hook record;
- a current-attempt private workflow artifact;
- stale forms of hook and artifact evidence.

The acceptance criterion is expressed as scheduler tests.

It requires a matching hook to accelerate ownership while a claim is pending.

It requires a valid current-attempt artifact to establish bounded fallback
ownership.

It requires stale-attempt hook and artifact evidence to be ignored.

No launcher, CLI claim producer, dashboard-label, or ticket-boundary change is
part of this ticket.

## Repository organization

The plugin scheduler currently lives primarily in
`crates/lisa-plugin/src/lib.rs`.

The repository context still describes a separate `scheduler.rs`, but the
current checkout has the scheduler state machine, polling, and native tests in
`lib.rs`.

Filesystem signal recognition and acquisition live in
`crates/lisa-plugin/src/signal.rs`.

Provider-specific Codex hook payload parsing lives in the `codex_ack` module.

The shared claim record lives in `crates/lisa-core/src/claim.rs`.

T-045-03-01 is committed at the current base and added claim ingestion and
claim admission to the plugin.

## Scheduler ownership state

`State::seat_assignments` is keyed by physical pane ID.

It is distinct from slot reservation and pane transition state.

`SeatAssignmentState` currently contains the pre-ownership states:

- `Starting`;
- `ResettingStartup`;
- `ReadyForAssignment`;
- `Delivering`;
- `AssignedPendingAck`;
- `Recovering`.

It also contains `Owned` and named terminal failure states.

`State::seat_is_owned` reports true only for the exact `Owned` variant.

`active_assignment_generation` returns a generation only for `Delivering`,
`AssignedPendingAck`, or `Recovering`.

That helper therefore defines which delivered states may accept ownership
evidence today.

Starting and ready states cannot become owned from claim or hook evidence.

## Lease authority

`State::current_leases` is the scheduler's current authority per ticket.

Each `AgentSlot` also retains its pane-specific `attempt_lease`.

Each running `Thread` retains its own `attempt_lease`.

The lease contains the ticket ID and monotonic attempt ID.

`AttemptLease::is_current` compares a candidate with the current authority.

The scheduler uses these duplicated references to bind pane routing, thread
execution, and filesystem output to one attempt.

Lease revocation removes predecessor authority before a replacement is used.

Stale evidence may remain on disk, but it must not pass the in-memory current
lease comparison.

## Exact assignment identity

`State::assignment_refs` retains the successfully published assignment for the
ticket's current attempt.

An `AssignmentRef` contains the attempt lease, immutable assignment path, and
nonce.

The nonce distinguishes multiple assignment publications even when ticket and
attempt identifiers alone would match.

Claims contain ticket ID, attempt ID, and nonce.

Hook payloads contain the tagged ticket ID and generation but not the nonce.

Artifacts are attributed by their private attempt directory and lease rather
than by an embedded claim record.

These inputs consequently have intentionally different identity strength.

## Claim path

`signal::SignalRequest::Claims` recognizes exact `pane-<u32>.claim` filenames.

The signal module deserializes the body as `AssignmentClaim`.

Recognized claim files are consumed once, including malformed bodies.

`State::check_claim_signals` runs the scheduler admission for each typed record.

`State::admit_assignment_claim` requires:

- a non-owned active delivered generation;
- the addressed pane's slot;
- matching slot ticket and claim ticket;
- matching state generation and claim attempt;
- matching slot lease and claim attempt;
- current lease authority;
- a retained assignment reference;
- matching retained lease and claim nonce.

On success it changes the pane assignment to `Owned`.

The claim consumer then bumps pane/thread activity and logs a claim-specific
information event.

In `poll_tick`, claims are consumed before hook acknowledgements and artifacts.

That order already gives the claim the first opportunity to perform the
pending-to-owned edge in a poll where multiple evidence forms coexist.

## Hook path

`signal::SignalRequest::CodexAcknowledgements` recognizes exact
`pane-<u32>.ack` filenames.

The body remains raw provider JSON at the signal boundary.

Recognized hook files are one-shot.

`State::check_codex_ack_signals` calls
`acknowledge_codex_assignment` for each pane-routed payload.

The acknowledgment method requires:

- a non-owned active delivered generation;
- the addressed pane's ticket and attempt lease;
- equality among ticket, state generation, and lease generation;
- current lease authority;
- a provider payload matching the ticket and generation tag.

On success it changes the assignment to `Owned`.

The consumer bumps activity and logs that the pane acknowledged its assignment.

The hook is therefore already sufficient to accelerate a pending ownership
transition.

It is processed after claims in the poll, so it is supplemental in ordering.

It remains useful when the claim signal has not arrived yet.

A hook for a prior generation fails the generation and current-lease checks.

A hook routed to a released or predecessor pane has no matching active slot
and also fails closed.

## Private artifact path

`State::attempt_work_dir` maps an `AttemptLease` to
`.lisa/attempts/<ticket>/<attempt>/work` in production.

This path is private staging for one scheduler attempt.

`State::check_artifact_advances` snapshots running threads as ticket, phase,
and optional source lease tuples.

For each current phase it chooses only the artifact that represents that phase
edge.

Research, Design, Structure, and Plan use their phase artifact filename.

Implement publishes `progress.md` for durability but only `review.md` is its
phase-completion artifact.

Review uses its configured artifact and completion-disposition checks later in
the completion path.

`State::admit_artifact` validates leased staging before publication.

It requires candidate ticket equality and exact current lease authority.

It then requires the expected staged path to be a regular file.

Only after those checks does it copy the bytes atomically to the canonical work
directory.

The unleased canonical branch exists for historical unit fixtures only when no
lease authority is registered.

A stale candidate returns an error before its staged bytes are inspected or
published.

The artifact checker logs rejected publication and does not advance the phase.

## Missing ownership edge

A successfully admitted private artifact currently advances workflow state but
does not affect `seat_assignments`.

Thus durable work from the exact current attempt can coexist with a seat still
shown as `Delivering` or another active pending state.

The scheduler already has enough information at that point to relate the
artifact's ticket and lease to the pane and active assignment generation.

The missing behavior is local to scheduler admission rather than filesystem
signal parsing.

No new artifact filename family is needed.

No artifact body schema exists or is required by the current workflow.

The expected phase artifact is a naturally bounded set selected by the current
phase, rather than an arbitrary file scan.

`progress.md` is handled specially and is not a phase edge.

That distinction matters because a living progress file can appear early and
does not by itself mean the Implement phase is complete.

## Poll ordering

The ownership-relevant production order is currently:

1. deliver assignments already ready before the poll;
2. process exact process-start evidence;
3. process exact shell-ready evidence;
4. process exact assignment claims;
5. process matching hook acknowledgements;
6. inspect and admit workflow artifacts;
7. evaluate idle, transitions, errors, and timeouts later.

The ordering already matches strongest-to-fallback evidence for the three
ownership inputs in this ticket.

An exact claim can own first.

If no exact claim owns, a matching hook can own before artifact observation.

If neither signal owns, the artifact checker is the final positive evidence
consumer before timeout policy.

## Existing test coverage

`delivered_assignment_becomes_owned_on_exact_claim_without_hook` constructs a
real scheduled Codex fixture, reaches `Delivering`, rejects a wrong nonce, and
then owns on the exact claim with no `.ack` file.

`test_dashboard_snapshot_shows_fresh_codex_handoff_states` demonstrates the
existing matching hook transition from delivering to owned.

`test_recycled_codex_ownership_requires_matching_ack_exactly_once` covers
stale ticket, stale generation, revoked authority, success, and duplicate hook
behavior.

`stale_attempt_cannot_keep_replacement_alive_or_publish_same_artifact` creates
predecessor and current private artifact directories.

It verifies predecessor heartbeat and artifact evidence neither refreshes nor
advances the replacement.

It verifies the current attempt's matching artifact is admitted and advances.

The split-brain timeline test also feeds stale pane signals and predecessor
private Review output after redispatch.

It verifies stale output remains private and cannot create completion.

No existing test connects successful artifact admission to seat ownership.

No single existing test states the full claim > hook > artifact observation
order as the ticket's evidence-tier contract.

## Constraints

Claude need not share Codex's claim handshake.

Existing Claude seats may already be marked owned through their unchanged
provider path.

Dashboard labels only reflect `SeatAssignmentState`; they are not evidence.

The next ticket owns new state variants and timeout semantics.

This ticket must not rename `Delivering` or change bounded retry behavior.

Stale private artifacts must remain on disk and unpublished rather than being
mistaken for current evidence.

Recognized signal files remain one-shot even when their evidence is rejected.

Ticket-owned source changes must be committed with `lisa commit-ticket` and an
exact repository-relative include path.
