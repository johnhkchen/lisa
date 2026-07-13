# Structure — T-045-04-01 clean-exit-revoke-attempt

## Change inventory

Modify one ticket-owned source file:

- `crates/lisa-plugin/src/lib.rs`

Create no production module.
Delete no file.
Change no public API, serialized schema, configuration, CLI, core type, or adapter.
Keep all RDSPI artifacts private under the current attempt work directory.

## `AttemptLifecycleEvent`

Extend the existing test-only enum with one variant:

```rust
CleanExitRequested {
    ticket_id: TicketId,
    pane_id: u32,
}
```

The variant represents successful-completion teardown submitting the resident
interactive client's graceful exit command.
It is observational only and compiled only for native tests.
It does not become scheduler authority or a production persistence format.

Keep existing variants unchanged:

- `LeaseRevoked`;
- `ShellInterrupted`;
- `ShellRelaunched`;
- `PaneFenced`;
- `SlotReleased`.

The new variant must not be emitted by hard-silence fencing or startup recovery.

## Generic release boundary

Keep `State::release_slot_for_ticket` structurally unchanged.
It remains the shared lease-revocation and slot-release primitive.
Its current callers in failure, reset, audit, and existing tests retain behavior.

It continues to own:

- `revoke_current_lease`;
- clearing slot ticket and attempt lease;
- removing seat ownership;
- clearing attention state;
- applying normal resident-session cooldown;
- idle pane renaming;
- `SlotReleased` observation and release activity.

No provider-specific process policy is added inside this generic function.

## Successful completion release

Add a private method beside `release_slot_for_ticket`:

```rust
fn release_completed_slot_for_ticket(&mut self, ticket_id: &TicketId)
```

### Pre-release observation

Before generic release, inspect the slot whose `ticket_id` matches.
Snapshot a pane ID only for a live, non-fenced Codex resident session.

Resolve Codex's adapter through `resolve_adapter_or_native` using the resident
client and the configured Lisa binary.
Read `adapter.exit_command()` before or after release; it owns the exit spelling.
Do not infer the command text elsewhere.

### Shared release delegation

Call `release_slot_for_ticket(ticket_id)` once for every invocation.
This is the authority edge.
It must happen before the clean-exit submission and before future scheduling.

### Codex exit mutation

If no eligible Codex pane was snapshotted, return after generic release.

Otherwise:

1. submit the exit command with `send_line_to_pane`;
2. find the now-unassigned slot by pane ID;
3. set `transition_state` to `WaitingForExit`;
4. set `transition_started_at` to the same captured `SystemTime::now()`;
5. set `has_session` to false;
6. clear `cooldown_until` so only exit readiness gates reuse;
7. preserve `last_client == Some(Codex)` until shell cleanup;
8. append `CleanExitRequested` in test builds;
9. log a concise activity message naming pane and completed ticket.

The helper does not:

- restore a ticket reservation;
- mint an attempt;
- write an assignment or lease marker;
- launch a provider;
- change the completed thread;
- change completion journal or provenance;
- modify Claude state.

## Completion result call site

In `State::handle_completion_result`, replace:

```rust
self.release_slot_for_ticket(&ticket_id_owned);
```

with:

```rust
self.release_completed_slot_for_ticket(&ticket_id_owned);
```

Keep surrounding order unchanged:

1. confirm completion journal;
2. rebuild durable DAG;
3. log completion transitions;
4. mark thread complete;
5. emit authoritative Done provenance;
6. run successful-completion slot release;
7. remove thread;
8. call `schedule_ready_tickets`.

The immediate scheduling call remains valuable: other idle panes may accept work.
The exiting pane itself is excluded because it is not in `TransitionState::Idle`.

## Existing unassigned exit transition

Do not change `check_transition_timeouts` behavior.
The existing `ExitReady { ticket_id: None }` branch is the completion helper's
consumer.

It will return the slot to an empty-shell representation:

- transition Idle;
- no transition timestamp;
- no live session;
- no resident client;
- no seat assignment;
- pane title `lisa · idle`.

Normal scheduling then sees an empty slot and uses a fresh launch.

## Boundary fixture placement

Add one native test in `crates/lisa-plugin/src/lib.rs` near the existing
Codex consecutive-reuse and completion-boundary scheduler tests.

Name it descriptively, for example:

```rust
codex_completion_exits_revokes_and_launches_next_fresh_tui
```

Add a small local fixture constructor only if it makes the test clearer.
Prefer existing helpers where they match the required state exactly.
Do not create an external shell fixture because the honest boundary is native/stub.

## Boundary fixture topology

Construct a temporary ticket directory with two Codex tickets:

- predecessor in Ready;
- successor in Ready with `depends_on: [predecessor]`.

Configure:

- client Codex;
- one global thread;
- one fresh physical pane;
- `wind_down_secs = 0`;
- short assignment acknowledgement timeout;
- temporary attempt, work, signal, and ticket paths.

Set permissions and slot discovery true.
Set a fixture Lisa binary so launch-script assertions are deterministic.

## Fixture lifecycle

### Initial launch

Call `schedule_ready_tickets`.
Assert only the predecessor reserves the pane.
Capture its lease, assignment nonce, assignment path, and launch script.
Advance the fresh Codex startup grace to assignment delivery.
Construct an exact `AssignmentClaim` from the captured identity.
Call `admit_assignment_claim` and require `Owned`.

### Completion boundary

Update the predecessor ticket to durable Done.
Refresh the fixture DAG.
Mark the predecessor thread complete if needed by the modeled cleanup.
Call `release_completed_slot_for_ticket`.
Remove the predecessor thread to mirror verified result handling.

Assert:

- predecessor is absent from `current_leases`;
- high-water retains its lease;
- slot has no ticket or attempt lease;
- seat assignment is absent;
- transition is `WaitingForExit`;
- no live session is published;
- deferred exit Enter is queued;
- lifecycle trace orders revoke, release, clean exit;
- activity log includes the clean completion exit.

### Late claim

Call `admit_assignment_claim` with the retained exact predecessor claim.
Require false and no ownership restoration.
Require predecessor authority remains absent.

### Scheduling while exiting

Call `schedule_ready_tickets`.
Require the successor is still unassigned.
Require no successor lease or assignment reference exists.
Require no fresh successor launch activity occurred.

### Clean shell and fresh next TUI

Backdate `transition_started_at` beyond `AGENT_EXIT_GRACE_SECS`.
Call `check_transition_timeouts`.
Require the pane is empty-shell Idle with no resident client.

Call `schedule_ready_tickets` again.
Require the successor reserves the same physical pane.
Require it owns a separately minted lease and separately published assignment.
Require a new launch script invokes `launch-codex` with the successor assignment.
Require the seat is `Starting`, not Owned.
Require the slot has a live fresh Codex session and Idle transition state.

Attempt the predecessor claim again and require rejection.

## Transcript shape

Print stable rows with a ticket-specific prefix under `--nocapture`.
Suggested fields:

```text
T0450401|boundary|step=claimed|ticket=...|pane=...|attempt=...
T0450401|boundary|step=revoked|ticket=...|late_claim=rejected
T0450401|boundary|step=exit-requested|pane=...|next_reserved=false
T0450401|boundary|step=shell-ready|resident=none
T0450401|boundary|step=fresh-launch|ticket=...|pane=...|state=starting
```

The text is diagnostic evidence.
Exact Rust assertions remain the enforceable contract.

## Unchanged boundaries

No `AgentAdapter` interface change.
No Codex command construction change.
No claim schema or CLI validation change.
No completion journal schema or reducer change.
No authoritative completion-count assertion; that belongs to the dependent ticket.
No Claude release, `/clear`, or ownership transition change.
No timeout, stale-thread, reset, recovery, or fencing policy change.
No real Codex or real Zellij execution in this ticket.
