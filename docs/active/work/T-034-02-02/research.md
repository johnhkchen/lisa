# Research: T-034-02-02 gate completion on current lease

## Ticket and workflow state

- The ticket is `docs/active/tickets/T-034-02-02.md`.
- Its declared phase is Research and its status is Open.
- T-034-02-01 is the direct dependency and has already bound Codex assignment
  acknowledgement generations to scheduler attempt leases.
- This ticket owns the completion admission boundary only.
- T-034-02-03 separately owns lease attribution for heartbeat and artifact
  publication, including the shared-worktree problem.
- T-034-02-04 separately owns attempt-attributed provenance and the single
  authoritative Done record.
- The RDSPI workflow requires artifacts for every phase and forbids this agent
  from editing ticket phase or status frontmatter.
- The worktree contains unrelated modified and untracked files. Ticket work
  must use exact path ownership and Lisa's isolated commit command.

## Lease model already present

- `lisa_core::types::AttemptLease` contains a ticket ID and monotonic attempt ID.
- `AttemptLease::mint` creates attempt 1 without a predecessor or a checked
  successor for the same ticket.
- `AttemptLease::is_current` requires exact equality with an optional current
  lease and returns false when current authority is absent.
- `State::lease_high_water` retains the latest minted identity for redispatch.
- `State::current_leases` contains only presently authorized attempts.
- Dispatch inserts one newly minted lease into both maps.
- Dispatch stamps the same lease on the assigned `AgentSlot` and logical
  `Thread`.
- Fresh Codex recovery mints and installs a successor lease before delivery.
- `release_slot_for_ticket` revokes current authority before exposing a ticket
  for later scheduling.
- Hard-silence recovery explicitly revokes, fences the pane, and then releases.

## Completion transaction already present

- `State::request_completion` is the single plugin admission point for Done.
- It deduplicates requests with `pending_completions`.
- It rejects tickets whose dependencies are not all Done.
- It resolves the ticket's real file path from the current DAG.
- It records prior phase and status so a pending Done scan can be masked.
- It builds a native `lisa complete-ticket` command.
- The command receives the repository root, ticket ID, completion message,
  exact ticket file, and exact ticket work directory.
- In production, the plugin launches the command through Zellij and attributes
  its result with the `lisa_completion` context key.
- In native tests, command construction failure is treated as a successful
  request seam, leaving an observable pending completion.
- No current code checks attempt identity before inserting that pending record
  or launching the native transaction.

## T-031 isolated completion behavior

- The native `complete-ticket` command prepares Done frontmatter and calls the
  T-031-01 isolated Git transaction.
- The transaction uses an alternate index and explicit include paths.
- It serializes snapshot, staging, tree creation, commit creation, guarded HEAD
  update, ordinary-index reconciliation, verification, and cleanup.
- `handle_completion_result` accepts only exit zero plus a 40- or 64-character
  hexadecimal commit ID on stdout.
- Failed command results remove the pending marker, rebuild the DAG, keep the
  thread and slot, and log a retryable error.
- Successful results rebuild the DAG and verify both phase and status are Done.
- Only after durable verification does the plugin complete the thread, emit
  Done provenance, release the slot, remove the thread, and schedule dependents.
- T-034-02-02 must leave this transaction and publication ordering unchanged
  for an admitted current attempt.

## Completion request sources

- `CompletionSource::Artifact` is used when Review's artifact is observed by
  `check_artifact_advances`.
- `CompletionSource::Idle` is used when an idle signal coincides with Review
  completion, including the Implement-to-Review fast path.
- `CompletionSource::Stopped(pane_id)` is used for a stopped Review session.
- `CompletionSource::Manual` is used by the mark-Done modal.
- `CompletionSource::ObservedDone` reconciles externally observed Done
  frontmatter for an active thread.
- These variants currently carry diagnostic origin, not a lease.
- Only `Stopped` carries a pane ID.
- Artifact existence is not currently attributable to a writer; that explicit
  gap is the subject of T-034-02-03.

## Available attempt identity at callers

- Every normally dispatched active `Thread` has `attempt_lease` populated.
- Artifact scanning iterates active threads and can clone that thread stamp.
- Idle scanning first resolves pane-scoped signals to an assigned slot, but it
  also supports legacy ticket-scoped idle filenames without a pane identity.
- A logical thread stamp is available for both idle forms.
- Stopped-signal handling resolves a concrete assigned pane and can use the
  corresponding slot stamp.
- Manual completion can resolve the active thread stamp for the selected
  ticket.
- Observed-Done reconciliation iterates active threads and can clone each
  thread stamp.
- Test fixtures often construct `Thread::new` and `AgentSlot` directly, leaving
  their optional lease fields empty because they predate lease enforcement.

## Stale completion failure mode

- Attempt N can retain a delayed route to `request_completion` after attempt
  N+1 becomes current.
- The current function checks only ticket identity, dependency state, and
  pending state.
- Therefore a request originating from N can launch `complete-ticket` while
  `current_leases[ticket]` is N+1.
- If that transaction succeeds, durable Done is published and the replacement
  is torn down despite the old attempt lacking authority.
- The isolated Git transaction protects repository/index integrity but does not
  know scheduler lease authority.
- Gating must happen before command launch; rejecting only the eventual command
  result would be too late because the commit could already exist.

## Relevant state boundaries

- The canonical authority check is the source lease's
  `is_current(current_leases.get(ticket))` result.
- Ticket IDs must agree as part of exact lease equality.
- Missing source identity must fail closed; absence cannot prove authority.
- Missing current authority must fail closed.
- A stale rejection must not insert `pending_completions`.
- A stale rejection must not build or launch the native completion command.
- It must not mutate ticket frontmatter, thread status, slot assignment,
  provenance, dependency readiness, or lease state.
- The rejection should be visible in the scheduler activity log.

## Caller-versus-boundary responsibilities

- Callers observe an event and identify the attempt they attribute it to.
- `request_completion` is responsible for comparing that source identity with
  scheduler authority.
- Keeping the source lease explicit prevents `request_completion` from silently
  substituting the replacement thread's identity for a stale event.
- It also gives T-034-02-03 a stable seam for supplying an artifact publication
  lease once artifact writes become attempt-attributable.
- Deriving the lease only inside `request_completion` from the current thread
  would not preserve event provenance: a stale event could be credited to the
  replacement automatically.

## Existing tests and likely impact

- Completion tests cover artifact, idle, stopped, manual, external-Done, failed
  transaction retry, dependency scheduling, and verified publication.
- Many fixtures model assigned work without installing `current_leases` or
  lease stamps.
- Once admission fails closed, completion fixtures that are intended to model
  real scheduled attempts must install a matching lease.
- A small test helper can stamp the registry, thread, and matching slot from one
  minted value, mirroring production dispatch.
- Existing non-completion fixtures can remain unleased.
- The key new regression should present a prior source lease while a successor
  is current, call the real request boundary, and assert rejection without a
  pending transaction.
- The same regression should then present the successor lease and assert the
  normal pending transaction is created.
- Existing verified-success coverage demonstrates that the admitted path still
  reaches the unchanged isolated transaction result publisher.

## Constraints and assumptions

- No lease persistence across plugin restarts is introduced here.
- No command-line lease argument or native transaction change is required: the
  authoritative decision occurs before process launch inside the scheduler.
- No signal payload or hook format changes belong to this ticket.
- Artifact writer attribution remains incomplete until T-034-02-03, but the
  completion API should be ready to accept that identity explicitly.
- Provenance remains ticket/thread based until T-034-02-04.
- Manual completion of an active ticket is an action on its active attempt and
  therefore must carry that attempt's current lease.
- A legacy or synthetic active thread with no lease is not authoritative and
  should no longer be permitted to complete a ticket.
- Current-attempt completion must preserve every T-031 ordering and retry
  guarantee without modification.
