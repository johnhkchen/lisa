# T-035-01-04 Research — bounded startup recovery

## Ticket boundary

The ticket begins in Research and completes the final acceptance point of story
`S-035-01`: a fresh native provider launch must not remain indefinitely in the
lease-scoped `Starting` state when its process-start signal never arrives.

The single acceptance criterion asks for a native test that withholds the signal and
establishes four observable facts:

- the wait is bounded;
- the seat leaves `Starting` by the deadline;
- the result is a named recovery or failure state, never `Owned`;
- polling cannot relaunch the provider without bound.

The ticket explicitly builds on E-033's finite fresh-session fallback. It does not own
the start-signal producer, the initial ownership gate, the launch transport, a real
Zellij PTY reproduction, or installed-provider validation.

## Predecessor state

T-035-01-01 added a provider-neutral native process-start producer. Claude and Codex
startup hooks write an attempt lease to `pane-<id>.started`.

T-035-01-02 made fresh launch delivery shell-safe and bounded by writing the complete
payload to an attempt-private launcher before sending a short command and Enter.

T-035-01-03 added `SeatAssignmentState::Starting { generation }`, classified all fresh
process routes, consumed `.started` files, and admitted only the exact current lease.

Immediately after a fresh dispatch, scheduler truth is therefore pending and visible.
The remaining gap is temporal: `Starting` contains identity but no time boundary.

## Assignment state model

`crates/lisa-plugin/src/lib.rs` owns the private assignment enum:

```rust
enum SeatAssignmentState {
    Starting { generation: u64 },
    AssignedPendingAck {
        generation: u64,
        ack_deadline: Option<SystemTime>,
    },
    Owned,
    Recovering {
        generation: u64,
        ack_deadline: Option<SystemTime>,
    },
    RecoveryFailed,
}
```

The map is keyed by physical pane ID. Absence means unassigned. The slot independently
retains ticket, attempt lease, provider, and transition information.

Only exact equality with `Owned` makes `seat_is_owned` true. `Starting`, E-033 pending,
recovery, and recovery failure all retain a reservation without claiming ownership.

`Starting` carries the current attempt generation. It deliberately does not participate
in `active_assignment_generation`, which is specific to generation-tagged recycled
Codex prompt acknowledgment and its recovery attempt.

## Exact start admission

`acknowledge_process_start` performs the sole `Starting -> Owned` edge. It requires:

- the pane's current assignment is `Starting`;
- the candidate attempt ID equals the starting generation;
- the pane resolves to a slot with a ticket and attempt lease;
- candidate ticket, slot ticket, and slot lease agree;
- the candidate remains current in `current_leases`.

Malformed, stale, cross-ticket, duplicate, or already-consumed starts cannot establish
ownership. `check_process_start_signals` removes recognized `.started` files before
attempting admission, so invalid signals are one-shot rather than replayed each poll.

The scanner runs immediately after heartbeat consumption near the top of `poll_tick`.
It therefore wins before the later timeout evaluators when a valid signal and an
expired deadline are both visible in the same tick.

## Fresh launch routes and timing boundaries

`schedule_ready_tickets` computes `fresh_launch` for three route shapes:

1. an empty pane receives a provider launch immediately;
2. a reusable shell receives a `FreshExec` launch immediately;
3. cross-provider recycling sends the resident provider's exit command first and
   launches the incoming provider later from `check_transition_timeouts`.

The first two routes have submitted the launcher by the time assignment state is
installed. The third is only reserved at that point; its provider process has not been
launched while the pane is `WaitingForExit`.

This distinction matters for any startup clock. Starting it at reservation time would
charge cross-provider exit grace against provider startup and could expire before the
incoming launch exists. The clock boundary must be actual fresh launch submission.

`send_line_to_pane` types the command immediately and queues Enter after
`ENTER_DELAY_SECS`. The existing E-033 acknowledgment helper adds that transport delay
to its configured timeout so the acceptance window does not expire on unsubmitted text.

## E-033 bounded fallback machinery

E-033 introduced `assignment_ack_timeout_secs`, defaulting to 30 and required to be
positive through the CLI configuration path. `PluginConfig::from_config_map` also
falls back to the finite default for missing, zero, or malformed direct plugin values.

`start_assignment_ack_wait` computes an absolute deadline from the injected current
time plus the configured wait and Enter delay. It currently arms only
`AssignedPendingAck` or `Recovering` states whose deadline is `None`.

`check_assignment_ack_timeouts_at` accepts an injected `SystemTime`, allowing native
tests to reach deadlines deterministically without sleeping. The production wrapper
uses `SystemTime::now()` and runs once per poll after signal and transport processing.

An expired reused-Codex pending assignment begins exactly one fresh-session recovery.
The recovery mints a successor lease, fences the predecessor, sends `/exit`, launches
once after exit grace, and arms one final acknowledgment deadline.

An expired recovery enters `RecoveryFailed`, fails the retained thread, adds an alert,
and tells the operator to reset the ticket. It does not release the slot or thread back
to automatic scheduling, which prevents infinite relaunch.

E-033's recovery states and acknowledgment generation are Codex-specific. Its helpers
refer to Codex prompts, native Codex adapter commands, and `.ack` payloads. The prior
ticket's handoff explicitly identifies startup timeout as an extension of `Starting`,
not a reinterpretation of those Codex prompt-ack fields.

## Existing failure and retry behavior

Generic `.error` handling can fail a thread, release its slot, remove it, and allow a
ready ticket to schedule again. That behavior is unsuitable as the missing-start
deadline because it can produce scheduler retries rather than a retained terminal
assignment outcome.

Broader session health and stale-thread timeouts are not start acknowledgment. They use
different clocks and can leave the dashboard semantically misleading for too long.

Keeping the ticket and slot associated in a terminal named startup state blocks the
normal idle-slot scheduler from selecting the physical pane for another automatic
attempt. An explicit ticket reset remains the operator-authorized retry mechanism.

## Dashboard boundary

`crates/lisa-plugin/src/ui.rs` exposes assignment state through
`SeatAssignmentStatus`. Current labels are `starting`, `assigned-pending-ack`, `owned`,
`recovering`, and `recovery-failed`.

`State::to_ui_state` converts every internal variant to the UI enum. Any new terminal
startup-specific variant therefore requires an exhaustive mapping and a stable label.
Pending states use yellow, owned uses green, recovery uses bright yellow, and failure
uses red.

The ticket's “named actionable” language and P4 operability mean internal state alone
is not the full existing pattern: E-033's terminal state is visible in the same seat
status column used for `starting`.

## Native test fixtures

`pane_name_schedule_state` builds a one-ticket DAG, one pane, finite plugin config,
signal directory, and scheduler permissions. With no resident provider it exercises a
real fresh native dispatch through `schedule_ready_tickets`.

`test_fresh_dispatch_becomes_owned_only_after_exact_process_start` already proves the
positive and rejection paths. It captures the scheduler-minted lease, observes
`Starting`, withholds ownership until a matching file exists, and snapshots dashboard
labels.

E-033 tests obtain stored deadlines from enum state and call
`check_assignment_ack_timeouts_at(deadline)`. The startup regression can follow this
deterministic pattern and additionally compare the recorded launch-event count before
and after repeated later timeout checks.

Native tests can inspect `sent_commands` or `ActivityEvent::SessionLaunch` history to
prove no additional process launch is submitted. No real timer or provider process is
needed for the acceptance boundary.

## Relevant file boundaries

The expected source seam is narrow:

- `crates/lisa-plugin/src/lib.rs`: startup deadline state, arming, evaluation, terminal
  failure transition, poll behavior, and native regression;
- `crates/lisa-plugin/src/ui.rs`: terminal startup status label/color and mapping target.

The existing `assignment_ack_timeout_secs` configuration already supplies the finite,
positive, documented bound. No new config pipeline is required unless startup needs a
distinct policy, which the ticket does not request.

No signal producer, adapter, CLI, launcher, ticket parser, or DAG module needs to change.

## Constraints and assumptions

- Process-start signals remain provider-neutral and exact-lease scoped.
- A valid start observed on the deadline tick must win over timeout.
- The wait begins when the fresh launch command is submitted, not at reservation.
- The configured E-033 bound is reusable for fresh provider acceptance.
- A missing first-launch signal should not manufacture `Owned`.
- Automatic relaunch is not required to satisfy a bounded terminal outcome.
- Any retry must remain operator-authorized through existing reset behavior.
- Concurrent working-tree changes outside the two ticket-owned source files must be
  preserved and excluded from ticket commits.

## Research conclusion

The code already has all primitives needed for a finite startup outcome: a positive
config value, injected-time deadline evaluation, exact signal-before-timeout poll
ordering, retained terminal failure precedent, and visible assignment statuses. The
missing pieces are a deadline attached to `Starting`, arming at actual fresh submission,
an explicit provider-neutral terminal transition, and a native no-signal/no-relaunch
regression.
