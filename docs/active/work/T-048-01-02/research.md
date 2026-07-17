# Research — T-048-01-02 park-instead-of-churn

## Ticket boundary

The ticket starts in Research and is the second task in story S-048-01.

Its predecessor, T-048-01-01, is complete and supplies two contracts:

- structured blocking Review dispositions in `lisa-core::disposition`;
- typed park/unpark rows in `lisa-core::provenance`.

This ticket owns scheduler policy that consumes those contracts.

The later S-048-02 work owns dashboard/CLI copy, an unblock command, execution
of world-owned checks, and authoring guidance. This ticket does not run checks.

The worktree contains unrelated Lisa-managed and ticket-owned changes. The
current task must restrict source commits to exact owned paths.

## Repository layout versus ticket wording

The ticket names `crates/lisa-plugin/src/scheduler.rs` as the scheduler file.

The current repository no longer has that file. Scheduler state and policy are
implemented in `crates/lisa-plugin/src/lib.rs`.

Relevant supporting modules are:

- `crates/lisa-core/src/disposition.rs` for parsed Review outcomes;
- `crates/lisa-core/src/provenance.rs` for append-only ledger rows;
- `crates/lisa-core/src/dag.rs` for scheduling eligibility;
- `crates/lisa-core/src/ticket.rs` for frontmatter mutation;
- `crates/lisa-core/src/types.rs` for tickets, threads, leases, and config;
- `crates/lisa-plugin/src/ui.rs` for board projection.

Plugin tests are primarily an inline `#[cfg(test)]` module in `lib.rs`.

## Structured Review block contract

`parse_review_disposition` reads and validates `review-disposition.json`.

`ReviewDisposition` has `Pass`, `Block`, and `Invalid` variants.

The `Block` variant carries:

- the original engineering `reason`;
- `remedy_owner: RemedyOwner`;
- a plain-language `ask`;
- optional concrete `steps`;
- an optional inert `check` string;
- an `unstructured` fallback marker.

`RemedyOwner` is the shared typed vocabulary: `Agent`, `Operator`, or `World`.

Missing or malformed block structure becomes an operator-owned unstructured
block. It does not become `Invalid`, so the churn-safe fallback is available to
the scheduler.

The parser never executes a `check`. World-owned check execution is absent from
the current scheduler and intentionally belongs to T-048-02-02.

## Review artifact admission

Each execution attempt writes artifacts beneath
`.lisa/attempts/<ticket>/<attempt>/work`.

`State::admit_artifact` verifies current attempt authority before copying an
artifact into the canonical `docs/active/work/<ticket>` directory.

`State::review_completion_inputs` admits both `review.md` and
`review-disposition.json`, then parses the canonical disposition.

This means an admitted structured block already has a durable carrier beside
the ticket. The scheduler does not need a second block document.

The canonical block remains available after its execution thread and lease are
released, including across scheduler restart.

## Current blocking behavior

`State::check_artifact_advances` scans running threads in a loop.

For a Review thread it admits `review.md`, computes `Phase::Done` as the next
phase, and calls the commit-gated completion dispatcher.

The completion path re-derives durable Review inputs. Only an exact `Pass`
authorizes completion.

A valid `Block` becomes `CompletionRejection::DispositionBlocked`.

The rejection is logged, but it does not alter ticket status, remove the
thread, release the slot, or establish any bounded recovery policy.

`review_completion_suppresses_finish_up` treats a valid block as a complete
Review protocol response, so the generic finish-up prompt is suppressed.

The thread nevertheless remains `Running`. Later error, timeout, or stale
reclaim paths can release it while leaving the ticket open and schedulable.

The next scheduler pass can therefore mint another attempt for the same Review
ticket. There is no block-owner distinction and no cross-attempt retry bound.

## Poll and scheduling order

`State::poll_tick` consumes lifecycle signals, checks artifacts, reconciles
Review completion, processes timeouts/failures, rebuilds the DAG, audits state,
and finally calls `schedule_ready_tickets`.

`schedule_ready_tickets` obtains `Dag::get_ready_tickets`, skips tickets with
live threads, enforces global/provider caps, selects a physical slot, mints an
attempt lease, writes assignment material, and creates a running `Thread`.

Running thread count enforces `max_threads`. A parked policy must release and
remove the running thread before ready tickets can claim those seats.

Slot release revokes the current lease, clears ticket/attempt assignment from
the slot, removes seat-assignment state, and leaves a reusable or recyclable
physical pane.

`lease_high_water` remains in memory after release, so same-loop redispatches
receive monotonically increasing attempt IDs.

## Existing durable scheduling exclusion

`Dag::can_start` rejects a ticket whose `TicketStatus` is `Blocked` before it
checks dependency ancestry.

`Dag::get_ready_tickets` uses this eligibility rule. A blocked ticket therefore
cannot be selected by `schedule_ready_tickets`.

`ticket::update_ticket_status` rewrites only the status field in YAML
frontmatter. It supports both `Blocked` and `Open`.

The ticket remains in the DAG when blocked. Its phase, dependency edges, title,
priority, and canonical work artifacts are not deleted.

The UI projects every DAG ticket into `ui::TicketNode`. A blocked ticket stays
visible as board state even when no `Thread` object exists for it.

This is distinct from the older `ThreadStatus::Parked` UI representation,
which retains a thread and pane identity. Retaining such a thread would make
unparking require scheduler cleanup and would conflict with seat release.

## Provenance contract from T-048-01-01

`ParkingTransitionType` currently contains `Park` and `Unpark`.

`ParkingTransitionRecord` carries:

- schema version;
- transition type;
- ticket ID;
- complete attempt lease;
- remedy owner;
- start/end epoch seconds;
- wall-clock seconds.

`append_parking_transition_record` writes one compact append-only JSONL row.

`ProvenanceLedgerRecord` replays assignment, parking, and execution records from
the same mixed ledger without changing older execution row shapes.

The plugin currently imports no parking record types and emits no park/unpark
rows. Its usage-attribution reader deliberately ignores parking records.

The existing parking shape records the final transition and elapsed interval,
but has no retry discriminator, retry ordinal, configured limit, or explicit
world recheck marker.

## Existing retry provenance

Normal execution teardown can append `ProvenanceRecord` with outcomes `Done`,
`Failed`, or `TimedOut`.

Assignment failures have a separate `AssignmentTransitionRecord` and reason.

An agent-authored Review block is neither a provider assignment failure nor a
timeout. Reusing either row would misclassify a deliberate block.

The ticket acceptance requires every agent-block retry and its bound to be
visible in provenance. The current park/unpark vocabulary cannot represent
that fact without an additive extension.

## State and restart boundaries

The scheduler has no persisted generic state store beyond ticket frontmatter,
attempt artifacts, completion journal, signals, and provenance.

A “fixed number per loop” can be represented by an in-memory counter keyed by
ticket ID. Restarting the loop naturally resets that bound, matching the stated
scope.

Parking itself must be durable in ticket frontmatter, because in-memory state
would disappear on restart and permit reseating.

Unparking is defined by changing status back to `open`. Since DAG eligibility
already consumes that field, scheduling does not require a separate allow-list.

An unpark provenance duration still needs the preceding park timestamp, owner,
and lease. Those facts can be reconstructed from the append-only ledger rather
than retained as scheduling authority.

## Constraints and invariants

- Only Lisa mutates ticket phase/status during agent execution.
- Parking must not write Done or bypass completion authority.
- Status must become durably blocked before the seat is released.
- A failed status write must leave the current attempt in place rather than
  silently dropping the ticket.
- Parked tickets must have no running thread and no assigned seat.
- Operator and world blocks park on their first admitted block.
- Agent blocks may retry only a small fixed number in one scheduler process.
- The retry counter is policy state, not ticket eligibility state.
- Unparking must use ordinary `status: open` DAG behavior.
- Provenance failure is observational and follows existing best-effort ledger
  behavior; it must be logged but cannot corrupt scheduling authority.
- World checks remain inert data in this ticket.
- Existing completion, lease fencing, assignment acknowledgement, and Done
  commit behavior must remain unchanged.

## Test infrastructure

Native plugin tests can call private `State` methods and use no-op Zellij host
functions.

Existing helpers build temporary ticket directories, scan them into a `Dag`,
create `AgentSlot` values, install current leases, and write attempt-local
Review dispositions.

`schedule_ready_tickets` is already exercised in native tests with stub panes.
It creates real assignment files while pane writes remain inert/observable.

The mixed-ledger test helpers deserialize `ProvenanceLedgerRecord`, so replay
assertions can inspect exact transition order and fields.

The acceptance replay can therefore use real scheduler state with two occupied
slots, two externally owned blocks, two queued tickets, private attempt
artifacts, and a temporary provenance ledger.

## Baseline conclusion

The churn is not caused by DAG readiness. It is caused by never converting a
valid completion-blocking verdict into the durable blocked status the DAG
already understands.

The smallest policy seam is between admitted Review evidence and the later
completion/timeout scheduling consequences.

Durable status, canonical structured disposition, released thread/slot state,
and append-only provenance together cover the ticket without changing the Done
state machine or executing external checks.
