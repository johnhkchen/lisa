# Research — T-045-03-01 claim is ownership proof

## Ticket boundary

T-045-03-01 is the first scheduler ticket in S-045-03.
It connects the already-published assignment claim to scheduler ownership.
The acceptance criterion requires a scheduler test with three observations:

- assignment delivery is visible;
- delivery alone remains not-owned;
- a valid claim alone promotes the seat to owned with no hook signal.

The story places hook evidence and artifact fallback in T-045-03-02.
It places the new delivered-awaiting-claim state and timeout behavior in
T-045-03-03.
This ticket therefore owns claim ingestion and the immediate claim-to-owned edge,
not the later evidence hierarchy or timeout redesign.

## Existing assignment states

`crates/lisa-plugin/src/lib.rs` contains the scheduler state machine.
The project layout documentation still names `scheduler.rs`, but scheduling remains
implemented in the large plugin `lib.rs` module.

`SeatAssignmentState` is scheduler-owned truth keyed by physical pane ID.
Its relevant variants are:

- `Starting`, before exact process-start evidence;
- `ReadyForAssignment`, after provider start and before chat submission;
- `Delivering`, after the bounded assignment reference is submitted;
- `AssignedPendingAck`, for existing reused-seat paths;
- `Recovering`, for the current bounded recovery path;
- `Owned`, after the current acknowledgement transition;
- named terminal startup, recovery, and delivery failures.

Absence from `seat_assignments` means the pane has no assignment.
The slot ticket ID is a reservation/routing value and does not itself prove
ownership.
`seat_is_owned` recognizes only the `Owned` variant.

## Existing delivery transition

`deliver_ready_assignments` snapshots all `ReadyForAssignment` seats at the start
of a poll and calls `deliver_assignment_to_pane` for each.
The delivery function resolves the retained assignment reference, sends the bounded
reference through the provider adapter, and inserts `Delivering` with:

- the active attempt generation;
- an acknowledgement deadline;
- the retry count.

It logs an information event naming the pane, ticket, generation, and retry.
It does not mark the seat owned.
The UI maps `Delivering` to a yellow `delivering` label.
This already provides the distinct delivered-side observation required by the
ticket.

## Current ownership transition

`acknowledge_codex_assignment` is the current pending-to-owned method.
It rejects an already-owned seat.
It obtains the expected generation through `active_assignment_generation`.
That helper admits `Delivering`, `AssignedPendingAck`, and `Recovering`.

The method then resolves the addressed physical slot and requires:

- a slot ticket ID;
- a slot attempt lease;
- equality between lease ticket and slot ticket;
- equality between lease attempt and state generation;
- exact currency in `current_leases`.

It parses provider-specific `UserPromptSubmit` JSON through `codex_ack`.
An exact tagged payload currently inserts `Owned` directly.
The signal consumer then bumps pane activity and logs that the pane acknowledged
its assignment.

This is the behavior E-045 intends to move away from as sole authority.
The hook path currently conflates provider prompt evidence with ownership.

## Assignment identity retained by the scheduler

`crates/lisa-plugin/src/assignment.rs` defines `AssignmentRef`.
It contains:

- a complete `AttemptLease`;
- a `u128` nonce;
- the exact durable assignment path.

`write_assignment` atomically publishes the complete assignment through a sibling
temporary and returns the reference only after rename succeeds.
`State::prepare_assignment` inserts that returned reference into
`assignment_refs`, keyed by ticket ID.

The map is documented as the exact successfully published assignment for the
ticket's current attempt.
Lease authority remains in `current_leases`.
The retained reference exists specifically so later delivery and claim evidence can
match the nonce-bearing assignment.

Old immutable assignment files may remain in an attempt directory.
Filesystem presence alone therefore cannot identify the live nonce.
The in-memory `assignment_refs` entry is the scheduler-side discriminator.

## Attempt lease authority

`State::current_leases` is the authoritative current-attempt registry.
`AttemptLease::is_current` requires exact equality with the registry entry.
`lease_high_water` persists generation history but does not authorize work.

The slot also retains `attempt_lease`.
The logical thread carries an attempt lease as well.
Existing signal admission uses the slot and current registry together so a
pane-routed record cannot claim another ticket or a stale attempt.

Lease revocation removes current authority while preserving high-water history.
Some durable pane markers can outlive in-memory authority.
The claim consumer must therefore perform the same in-memory current-lease check as
heartbeat, process-start, and hook acknowledgement consumers.

## Claim producer contract

T-045-01-02 added `lisa_core::claim::AssignmentClaim`.
The shared JSON payload contains:

- `ticket_id: TicketId`;
- `attempt_id: u64`;
- `nonce: u128`.

The CLI command validates the claim against the durable pane lease marker and the
deterministic assignment file, rereads the marker, then atomically publishes:

`.lisa/signals/pane-{pane_id}.claim`

The pane ID is carried by the strict filename rather than the payload.
The producer is intentionally not final authority because it cannot inspect the
plugin's `current_leases` or retained `assignment_refs` maps.

The predecessor handoff explicitly requires the scheduler to compare:

- strict pane routing;
- slot lease;
- current lease;
- retained assignment lease;
- retained assignment nonce;
- claim ticket, attempt, and nonce.

## Signal ingestion boundary

`crates/lisa-plugin/src/signal.rs` normalizes filesystem records.
`SignalRequest` enumerates one request per consumer family.
`SignalRecord` keeps typed lease payloads, raw provider payloads, and presence-only
signals distinct.

Strict pane families recognize exactly `pane-<u32>.<suffix>`.
Recognized one-shot records are deleted during ingestion, before scheduler
admission.
Malformed typed payloads are also consumed once.
Invalid filenames remain untouched because no consumer owns them.

There is no `Claims` request and no `Claim` record yet.
The core claim type is already available to the plugin crate.
Claim JSON can follow the typed lease ingestion pattern while retaining its own
record variant.

`clear_pane_lifecycle_signals` removes known pane-scoped runtime evidence during
attempt resets.
Its suffix list does not yet include `claim`.
Without adding it, an old unconsumed claim may remain across a pane lifecycle reset;
authoritative admission would still reject it, but cleanup would lag the other
one-shot assignment evidence.

## Poll ordering

`poll_tick` currently processes the assignment boundary in this order:

1. heartbeat signals;
2. awaiting-human signals;
3. delivery of seats already ready at the poll start;
4. process-start signals;
5. shell-ready signals;
6. Codex acknowledgement signals;
7. artifact and idle progression;
8. transition and error signals;
9. timeout fallbacks.

Delivery before signal consumers means a claim already present for a ready seat can
be considered after the state becomes `Delivering` in the same tick.
Process-start evidence remains observable as `ReadyForAssignment` for one complete
boundary because delivery precedes its ingestion.
Acknowledgement admission runs before deadline evaluation so exact evidence wins at
the deadline.

Two tests pin portions of this call order by reading `lib.rs` source text:

- `signal_consumer_characterization.rs`;
- `signal_ingestion_regression.rs`.

Adding a consumer requires updating both ordered lists if it participates in the
poll boundary.

## Dashboard observability

`State::to_ui_state` reduces internal states to `ui::SeatAssignmentStatus`.
`Delivering` and `Owned` already remain distinct values.
The UI labels are `delivering` and `owned` with yellow and green colors.
Scheduler tests use `dashboard_thread_row` to strip ANSI codes and assert visible
state labels.

The ticket does not require a new dashboard state.
S-045-03 explicitly says dashboard labels report transitions but never substitute
for them.
The existing output is sufficient if the underlying scheduler transition changes
only after valid claim admission.

## Test organization

Native plugin tests are embedded in `lib.rs` and split test modules under
`crates/lisa-plugin/src/tests`.
`signal_consumer_characterization.rs` already exercises each signal consumer with a
real temporary signal directory and directly constructed scheduler state.
It has helpers for installing a current attempt and checking activity effects.

The large `lib.rs` suite contains higher-level scheduler fixtures:

- `pane_name_schedule_state` constructs a real DAG/slot/attempt environment;
- `dashboard_thread_row` observes rendered state;
- fresh dispatch tests already walk Starting → Ready → Delivering → Owned;
- current tests create synthetic hook payloads to perform the final promotion.

The ticket acceptance sequence fits the higher-level fixture because it needs both
delivery and scheduler output.
Ingestion shape and one-shot behavior fit the split signal tests.

## Repository and workflow constraints

The worktree contains Lisa-managed provenance/completion state and materialized
epic, story, and ticket files unrelated to this source change.
They must remain untouched and excluded from ticket commits.

Phase artifacts belong only under:

`.lisa/attempts/T-045-03-01/1/work`

Ticket-owned source changes must be committed through `lisa commit-ticket` with
exact repository-relative paths.
Ordinary Git staging and commits are prohibited.

## Constraints surfaced

- The claim payload schema is already shared and should not be duplicated.
- The strict filename supplies pane routing; the payload supplies assignment identity.
- Final admission must compare both slot/current lease and retained nonce identity.
- A recognized claim is one-shot even when malformed or rejected.
- Delivery is already a real scheduler state and UI-visible.
- The claim must be accepted without any hook file.
- Artifact fallback and hook acceleration belong to the next ticket.
- The delivered-awaiting-claim state and no-reinjection timeout belong to the ticket after that.
- Existing lease fencing, process-start ordering, and cleanup behavior must remain intact.
- No live Codex or Zellij process is needed for this fixture-proven scheduler ticket.
